use std::{
    collections::HashMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU32, Ordering},
    },
};
use log::{Level, Record};
use rune_runtime::{CallContext, Function, Image, Output, Registrar};
use anyhow::{Context, Error};
use runic_types::SerializableRecord;

type OutputFactory =
    dyn Fn() -> Result<Box<dyn Output>, Error> + Send + Sync + 'static;
type LogFunc =
    dyn Fn(&log::Record) -> Result<(), Error> + Sync + Send + 'static;

#[derive(Clone)]
pub struct BaseImage {
    serial: Arc<OutputFactory>,
    log: Arc<LogFunc>,
}

impl Image for BaseImage {
    fn initialize_imports(self, registrar: &mut dyn Registrar) {
        let ids = Identifiers::default();
        let outputs = Arc::new(Mutex::new(HashMap::new()));

        registrar.register_function("env", "_debug", log(&self.log));

        let output_factories = Outputs {
            serial: Arc::clone(&self.serial),
        };
        registrar.register_function(
            "env",
            "request_output",
            request_output(&ids, &outputs, output_factories),
        );
        registrar.register_function(
            "env",
            "consume_output",
            consume_output(&outputs),
        );
    }
}

impl Default for BaseImage {
    fn default() -> Self {
        BaseImage {
            serial: Arc::new(|| anyhow::bail!("Unsupported")),
            log: Arc::new(|record| {
                log::logger().log(record);
                Ok(())
            }),
        }
    }
}

fn consume_output(
    outputs: &Arc<Mutex<HashMap<u32, Box<dyn Output>>>>,
) -> Function {
    let outputs = Arc::clone(outputs);

    Function::new(
        move |ctx: &dyn CallContext, (id, address, len): (u32, u32, u32)| {
            let data = ctx.memory(address, len).context("Bad input pointer")?;
            let mut outputs = outputs.lock().unwrap();
            let output = outputs.get_mut(&id).context("Invalid output")?;
            output.consume(data)
        },
    )
}

struct Outputs {
    serial: Arc<OutputFactory>,
}

fn log(log: &Arc<LogFunc>) -> Function {
    let log = Arc::clone(log);

    Function::new(move |ctx: &dyn CallContext, (msg, len): (u32, u32)| {
        let msg = ctx.utf8_str(msg, len)?;

        // this is a little more verbose than normal because we want to try
        // *really* hard to log messages and abort on errors.
        match serde_json::from_str::<SerializableRecord>(msg) {
            Ok(r) => {
                r.with_record(|record| log(record))?;

                if r.level == Level::Error {
                    // Make sure we abort on error
                    return Err(Error::msg(r.message.into_owned()));
                }
            },
            Err(_) => {
                log(&Record::builder().args(format_args!("{}", msg)).build())?;
            },
        };

        Ok(())
    })
}

#[derive(Default, Debug, Clone)]
struct Identifiers(Arc<AtomicU32>);

impl Identifiers {
    fn next(&self) -> u32 { self.0.fetch_add(1, Ordering::SeqCst) }
}

fn request_output(
    ids: &Identifiers,
    outputs: &Arc<Mutex<HashMap<u32, Box<dyn Output>>>>,
    constructors: Outputs,
) -> Function {
    let ids = ids.clone();
    let outputs = Arc::clone(outputs);

    Function::new(move |_: &dyn CallContext, (output_type,): (u32,)| {
        let output = match output_type {
            runic_types::outputs::SERIAL => (constructors.serial)()
                .context("Unable to create a new SERIAL output")?,
            _ => anyhow::bail!("Unknown output type: {}", output_type),
        };

        let id = ids.next();
        log::debug!("Output {} = {:?}", id, output);
        outputs.lock().unwrap().insert(id, output);

        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use std::{cell::UnsafeCell, collections::HashSet};
    use rune_runtime::WasmValue;
    use super::*;

    #[derive(Default, Debug)]
    struct DummyOutput(Arc<AtomicU32>);

    impl rune_runtime::Output for DummyOutput {
        fn consume(&mut self, _: &[u8]) -> Result<(), Error> {
            self.0.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    #[derive(Default, Debug)]
    struct DummyCallContext {
        memory: UnsafeCell<Vec<u8>>,
    }

    impl DummyCallContext {
        pub fn with_data(address: usize, data: &[u8]) -> Self {
            let mut memory = vec![0; address + data.len()];
            memory[address..address + data.len()].copy_from_slice(data);

            DummyCallContext {
                memory: UnsafeCell::new(memory),
            }
        }
    }

    impl CallContext for DummyCallContext {
        fn memory(&self, address: u32, len: u32) -> Result<&[u8], Error> {
            let start = address as usize;
            let end = start + len as usize;
            let memory = unsafe { &*self.memory.get() };
            memory.get(start..end).context("Out of bounds")
        }

        unsafe fn memory_mut(
            &self,
            address: u32,
            len: u32,
        ) -> Result<&mut [u8], Error> {
            let start = address as usize;
            let end = start + len as usize;
            let memory = &mut *self.memory.get();
            memory.get_mut(start..end).context("Out of bounds")
        }
    }

    #[test]
    fn invoke_output() {
        let calls = Arc::new(AtomicU32::new(0));
        let ctx = DummyCallContext::default();
        let d = DummyOutput(Arc::clone(&calls));
        let mut outputs = HashMap::new();
        outputs.insert(3_u32, Box::new(d) as Box<dyn Output>);
        let outputs = Arc::new(Mutex::new(outputs));
        let func = consume_output(&outputs);

        let ret = func
            .call(
                &ctx,
                &[WasmValue::I32(3), WasmValue::I32(0), WasmValue::I32(0)],
            )
            .unwrap();

        assert!(ret.is_empty());
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    fn mock_log() -> (Arc<Mutex<Vec<SerializableRecord<'static>>>>, Arc<LogFunc>)
    {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let calls2 = Arc::clone(&calls);

        let log_func: Arc<LogFunc> = Arc::new(move |r: &Record| {
            let record = SerializableRecord::from(r).into_owned();
            calls2.lock().unwrap().push(record);
            Ok(())
        });

        (calls, log_func)
    }

    #[test]
    fn call_log_function() {
        let record = SerializableRecord {
            message: "Hello, world".into(),
            ..Default::default()
        };
        let serialized = serde_json::to_vec(&record).unwrap();
        let ctx = DummyCallContext::with_data(42, &serialized);
        let (calls, log_func) = mock_log();
        let func = log(&log_func);
        let args =
            &[WasmValue::I32(42), WasmValue::I32(serialized.len() as i32)];

        let ret = func.call(&ctx, args).unwrap();

        assert!(ret.is_empty());
        let got = calls.lock().unwrap();
        assert_eq!(got.len(), 1);
        assert_eq!(&got[0], &record);
    }

    #[derive(Debug, Default)]
    struct Registrar {
        functions: HashMap<(String, String), Function>,
    }

    impl Registrar {
        fn keys(&self) -> HashSet<(&str, &str)> {
            let mut keys = HashSet::new();

            for (ns, name) in self.functions.keys() {
                keys.insert((ns.as_str(), name.as_str()));
            }

            keys
        }
    }

    impl rune_runtime::Registrar for Registrar {
        fn register_function(
            &mut self,
            namespace: &str,
            name: &str,
            function: Function,
        ) {
            self.functions
                .insert((namespace.to_string(), name.to_string()), function);
        }
    }

    #[test]
    fn functions_are_registered() {
        let mut registrar = Registrar::default();
        let image = BaseImage::default();

        image.initialize_imports(&mut registrar);

        let got = registrar.keys();
        let should_be: HashSet<_> = vec![
            ("env", "_debug"),
            ("env", "request_output"),
            ("env", "consume_output"),
        ]
        .into_iter()
        .collect();
        assert_eq!(got, should_be);
    }
}
