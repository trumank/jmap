use anyhow::{bail, Result};
use clap::Parser;
use dumper::{Input, StructInfo};
use std::{collections::BTreeMap, path::PathBuf};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Dump from process ID
    #[arg(long, short, group = "input")]
    process: Option<i32>,

    /// Dump from minidump
    #[arg(long, short, group = "input")]
    dump: Option<PathBuf>,

    /// Struct layout info .json (from pdb_dumper)
    #[arg(index = 1)]
    struct_info: PathBuf,

    /// Output dump .json path
    #[arg(index = 2)]
    output: PathBuf,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let input = match (cli.process, cli.dump) {
        (Some(number), None) => Input::Process(number),
        (None, Some(path)) => Input::Dump(path),
        (None, None) => {
            bail!("Error: Requires --process or --dump");
        }
        (Some(_), Some(_)) => {
            bail!("Error: Must specify either --process OR --dump");
        }
    };

    enum OutputType {
        Json,
        Usmap,
    }

    let output_type = match cli.output.extension().and_then(|e| e.to_str()) {
        Some("json") => OutputType::Json,
        Some("usmap") => OutputType::Usmap,
        _ => bail!("Error: Expected .json or .usmap output type"),
    };

    let struct_info: Vec<StructInfo> = serde_json::from_slice(&std::fs::read(cli.struct_info)?)?;

    let objects = dumper::dump(input, struct_info)?;

    match output_type {
        OutputType::Json => {
            std::fs::write(cli.output, serde_json::to_vec(&objects)?)?;
        }
        OutputType::Usmap => {
            let usmap = into_usmap(&objects);
            usmap.write(&mut std::io::BufWriter::new(std::fs::File::create(
                cli.output,
            )?))?;
        }
    }

    Ok(())
}

fn obj_name(path: &str) -> &str {
    path.rsplit(['/', '.', ':']).next().unwrap()
}

fn into_usmap(objects: &BTreeMap<String, ue_reflection::ObjectType>) -> usmap::Usmap {
    let mut enums = vec![];
    let mut structs = vec![];

    for (path, obj) in objects {
        if let Some(s) = obj.get_struct() {
            let mut properties = vec![];
            let mut index = 0;
            for prop in &s.properties {
                properties.push(into_usmap_prop(index, prop));
                index += prop.array_dim;
            }
            structs.push(usmap::Struct {
                name: obj_name(path).to_string(),
                super_struct: s.super_struct.as_ref().map(|s| obj_name(s).to_string()),
                properties,
            });
        } else if let Some(e) = obj.get_enum() {
            let prefix = format!("{}::", obj_name(path));
            let mut entries = vec![];
            for (name, _value) in &e.names {
                let variant_name = if let Some(variant_name) = name.strip_prefix(&prefix) {
                    variant_name
                } else {
                    assert!(!name.contains("::"), "enum prefix was not stripped");
                    name
                };
                entries.push(variant_name.to_string());
            }
            enums.push(usmap::Enum {
                name: obj_name(path).to_string(),
                entries,
            });
        }
    }

    usmap::Usmap {
        enums,
        structs,
        cext: None,
        eatr: None,
        envp: None,
        ppth: None,
    }
}

fn into_usmap_prop(index: usize, prop: &ue_reflection::Property) -> usmap::Property {
    usmap::Property {
        name: prop.name.clone(),
        array_dim: prop.array_dim.try_into().unwrap(),
        index: index.try_into().unwrap(),
        inner: into_usmap_prop_inner(&prop.r#type),
    }
}

fn into_usmap_prop_inner(prop: &ue_reflection::PropertyType) -> usmap::PropertyInner {
    use ue_reflection::PropertyType as PT;
    use usmap::PropertyInner as PI;
    match &prop {
        PT::Struct { r#struct } => PI::Struct {
            name: obj_name(r#struct).to_string(),
        },
        PT::Str => PI::Str,
        PT::Name => PI::Name,
        PT::Text => PI::Text,
        // TODO distinguish between sparse/inline?
        PT::MulticastInlineDelegate => PI::MulticastDelegate,
        PT::MulticastSparseDelegate => PI::MulticastDelegate,
        PT::Delegate => PI::Delegate,
        PT::Bool {
            field_size: _,
            byte_offset: _,
            byte_mask: _,
            field_mask: _,
        } => PI::Bool,
        PT::Array { inner } => PI::Array {
            inner: into_usmap_prop_inner(&inner.r#type).into(),
        },
        PT::Enum { container, r#enum } => PI::Enum {
            inner: into_usmap_prop_inner(&container.r#type).into(),
            name: r#enum
                .as_ref()
                .map(|e| obj_name(e))
                .unwrap_or("None")
                .to_string(),
        },
        PT::Map {
            key_prop,
            value_prop,
        } => PI::Map {
            key: into_usmap_prop_inner(&key_prop.r#type).into(),
            value: into_usmap_prop_inner(&value_prop.r#type).into(),
        },
        PT::Set { key_prop } => PI::Set {
            key: into_usmap_prop_inner(&key_prop.r#type).into(),
        },
        PT::Float => PI::Float,
        PT::Double => PI::Double,
        PT::Byte { r#enum } => {
            // usmap special cases ByteProperty to transform into EnumProperty if enum member is populated
            if let Some(e) = r#enum {
                PI::Enum {
                    inner: PI::Byte.into(),
                    name: obj_name(e).to_string(),
                }
            } else {
                PI::Byte
            }
        }
        PT::UInt16 => PI::UInt16,
        PT::UInt32 => PI::UInt32,
        PT::UInt64 => PI::UInt64,
        PT::Int8 => PI::Int8,
        PT::Int16 => PI::Int16,
        PT::Int => PI::Int,
        PT::Int64 => PI::Int64,
        PT::Object { class: _ } => PI::Object,
        PT::WeakObject { class: _ } => PI::WeakObject,
        PT::SoftObject { class: _ } => PI::SoftObject,
        PT::LazyObject { class: _ } => PI::LazyObject,
        PT::Interface { class: _ } => PI::Interface,
        PT::FieldPath => PI::FieldPath,
        PT::Optional { inner } => PI::Optional {
            inner: into_usmap_prop_inner(&inner.r#type).into(),
        },
    }
}
