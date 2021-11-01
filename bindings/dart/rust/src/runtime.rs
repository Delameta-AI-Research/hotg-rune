use core::slice;
use std::{
    ops::{Deref, DerefMut},
    os::raw::c_char,
    ptr,
};

use crate::{Error, Image};

/// The Rune runtime.
#[derive(Debug)]
pub struct Runtime {
    inner: hotg_rune_runtime::wasm3::Runtime,
}

impl Runtime {
    fn raw(inner: hotg_rune_runtime::wasm3::Runtime) -> *mut Self {
        Box::into_raw(Box::new(Runtime { inner }))
    }
}

impl Deref for Runtime {
    type Target = hotg_rune_runtime::wasm3::Runtime;

    fn deref(&self) -> &Self::Target { &self.inner }
}

impl DerefMut for Runtime {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.inner }
}

#[no_mangle]
pub unsafe extern "C" fn rune_runtime_create(
    wasm: *const c_char,
    wasm_len: usize,
    image: *mut Image,
    runtime: *mut *mut Runtime,
) -> *mut Error {
    ensure!(!image.is_null());
    ensure!(!runtime.is_null());
    ensure!(!wasm.is_null());

    let image = Box::from_raw(image);
    let wasm = slice::from_raw_parts(wasm as *const u8, wasm_len);

    match hotg_rune_runtime::wasm3::Runtime::load(wasm, image.into_inner()) {
        Ok(rt) => {
            runtime.write(Runtime::raw(rt));
            ptr::null_mut()
        },
        Err(e) => Error::new_ptr(e),
    }
}

#[no_mangle]
pub unsafe extern "C" fn rune_runtime_call(
    runtime: *mut Runtime,
) -> *mut Error {
    ensure!(!runtime.is_null());
    let runtime = &mut *runtime;

    match runtime.call() {
        Ok(_) => ptr::null_mut(),
        Err(e) => Error::new_ptr(e),
    }
}

#[no_mangle]
pub unsafe extern "C" fn rune_runtime_free(runtime: *mut Runtime) {
    if runtime.is_null() {
        return;
    }

    let _ = Box::from_raw(runtime);
}
