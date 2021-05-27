use std::{
    collections::{BTreeMap, HashMap},
    path::PathBuf,
};
use anyhow::{Context, Error};
use build_info::BuildInfo;
use rune_syntax::{
    hir::{HirId, Rune, SourceKind},
    yaml::{Type, Value},
};
use serde::{Serialize, Serializer};
use strum::VariantNames;
use wasmparser::{Parser, Payload};
use crate::Format;

#[derive(Debug, Clone, PartialEq, structopt::StructOpt)]
pub struct Inspect {
    #[structopt(
        short,
        long,
        help = "The format to use when printing output",
        default_value = "text",
        possible_values = Format::VARIANTS,
        parse(try_from_str)
    )]
    format: Format,
    #[structopt(help = "The Rune to inspect", parse(from_os_str))]
    rune: PathBuf,
}

impl Inspect {
    pub fn execute(self) -> Result<(), Error> {
        let wasm = std::fs::read(&self.rune).with_context(|| {
            format!("Unable to read \"{}\"", self.rune.display())
        })?;
        let meta = Metadata::from_custom_sections(wasm_custom_sections(&wasm))
            .context("unable to parse the Rune's metadata")?;

        match self.format {
            Format::Json => {
                let s = serde_json::to_string_pretty(&meta)
                    .context("Unable to format the metadata as JSON")?;
                println!("{}", s);
            },
            Format::Text => todo!(),
        }

        Ok(())
    }
}

#[derive(Debug, Clone, serde::Serialize)]
struct Metadata {
    version: Option<BuildInfo>,
    rune: Option<SimplifiedRune>,
}

impl Metadata {
    fn from_custom_sections<'a>(
        sections: impl Iterator<Item = CustomSection<'a>>,
    ) -> Result<Self, Error> {
        let mut version = None;
        let mut graph = None;

        for section in sections {
            match section.name {
                "rune_graph" => {
                    let rune: Rune = serde_json::from_slice(section.data)
                        .context("Unable to deserialize the graph")?;
                    graph = Some(SimplifiedRune::from(rune));
                },
                "rune_version" => {
                    version = Some(
                        serde_json::from_slice(section.data)
                            .context("Unable to deserialize the version")?,
                    );
                },
                _ => {},
            }
        }

        Ok(Metadata {
            version,
            rune: graph,
        })
    }
}

#[derive(Debug, Clone, serde::Serialize)]
struct SimplifiedRune {
    capabilities: BTreeMap<String, SimplifiedCapability>,
}

impl From<Rune> for SimplifiedRune {
    fn from(rune: Rune) -> Self {
        let mut capabilities = BTreeMap::new();

        for (&id, node) in &rune.stages {
            let name = rune.names[id].to_string();
            let outputs = node
                .output_slots
                .iter()
                .map(|slot| rune.slots[slot].element_type)
                .map(|type_id| resolve_type(&rune, type_id))
                .collect();

            match &node.stage {
                rune_syntax::hir::Stage::Source(rune_syntax::hir::Source {
                    kind,
                    parameters,
                }) => {
                    let kind = kind.clone();
                    let parameters = parameters.clone();
                    capabilities.insert(
                        name,
                        SimplifiedCapability {
                            capability_type: kind,
                            parameters,
                            outputs,
                        },
                    );
                },
                rune_syntax::hir::Stage::Sink(_) => {},
                rune_syntax::hir::Stage::Model(_) => {},
                rune_syntax::hir::Stage::ProcBlock(_) => {},
            }
        }

        SimplifiedRune { capabilities }
    }
}

fn resolve_type(rune: &Rune, type_id: HirId) -> Type {
    let (primitive, dims) = match &rune.types[&type_id] {
        rune_syntax::hir::Type::Primitive(p) => (p, vec![1]),
        rune_syntax::hir::Type::Buffer {
            underlying_type,
            dimensions,
        } => match &rune.types[underlying_type] {
            rune_syntax::hir::Type::Primitive(p) => (p, dimensions.clone()),
            _ => unreachable!(),
        },
        rune_syntax::hir::Type::Unknown | rune_syntax::hir::Type::Any => {
            unreachable!("All types should have been resolved")
        },
    };

    Type {
        name: primitive.rust_name().to_string(),
        dimensions: dims,
    }
}

#[derive(Debug, Clone, serde::Serialize)]
struct SimplifiedCapability {
    #[serde(serialize_with = "serialize_source_kind")]
    capability_type: SourceKind,
    outputs: Vec<Type>,
    parameters: HashMap<String, Value>,
}

fn serialize_source_kind<S: Serializer>(
    kind: &SourceKind,
    ser: S,
) -> Result<S::Ok, S::Error> {
    kind.to_string().serialize(ser)
}

fn wasm_custom_sections(
    wasm: &[u8],
) -> impl Iterator<Item = CustomSection<'_>> + '_ {
    Parser::default()
        .parse_all(wasm)
        .filter_map(Result::ok)
        .filter_map(|payload| match payload {
            Payload::CustomSection { name, data, .. } => {
                Some(CustomSection { name, data })
            },
            _ => None,
        })
}

#[derive(Debug, Copy, Clone, PartialEq)]
struct CustomSection<'a> {
    name: &'a str,
    data: &'a [u8],
}
