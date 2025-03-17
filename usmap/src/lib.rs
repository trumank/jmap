mod gen;

use std::io::Read;

use anyhow::{bail, Context, Result};

use byteorder::{ReadBytesExt, LE};
use serde::Serialize;
use tracing::instrument;

#[derive(Debug, Clone, strum::FromRepr)]
#[repr(u8)]
enum EPropertyType {
    ByteProperty,
    BoolProperty,
    IntProperty,
    FloatProperty,
    ObjectProperty,
    NameProperty,
    DelegateProperty,
    DoubleProperty,
    ArrayProperty,
    StructProperty,
    StrProperty,
    TextProperty,
    InterfaceProperty,
    MulticastDelegateProperty,
    WeakObjectProperty,
    LazyObjectProperty,
    AssetObjectProperty,
    SoftObjectProperty,
    UInt64Property,
    UInt32Property,
    UInt16Property,
    Int64Property,
    Int16Property,
    Int8Property,
    MapProperty,
    SetProperty,
    EnumProperty,
    FieldPathProperty,
    EnumAsByteProperty,

    Unknown = 0xFF,
}

#[derive(Debug, Clone, Serialize)]
pub enum PropertyInner {
    Byte,
    Bool,
    Int,
    Float,
    Object,
    Name,
    Delegate,
    Double,
    Array {
        inner: Box<PropertyInner>,
    },
    Struct {
        name: String,
    },
    Str,
    Text,
    Interface,
    MulticastDelegate,
    WeakObject,
    LazyObject,
    AssetObject,
    SoftObject,
    UInt64,
    UInt32,
    UInt16,
    Int64,
    Int16,
    Int8,
    Map {
        key: Box<PropertyInner>,
        value: Box<PropertyInner>,
    },
    Set {
        key: Box<PropertyInner>,
    },
    Enum {
        inner: Box<PropertyInner>,
        name: String,
    },
    FieldPath,
    EnumAsByte,
}

#[derive(Debug, Clone, Serialize)]
pub struct Usmap {
    //names: &'n [String],
    pub enums: Vec<Enum>,
    pub structs: Vec<Struct>,
    pub cext: Option<ExtCext>,
    pub ppth: Option<ExtPpth>,
    pub eatr: Option<ExtEatr>,
    pub envp: Option<ExtEnvp>,
}
#[derive(Debug, Clone, Serialize)]
pub struct Header {
    pub magic: u16,
    pub version: u8,
}
#[derive(Debug, Clone, Serialize)]
pub struct Struct {
    pub name: String,
    pub super_struct: Option<String>,
    pub properties: Vec<Property>,
}
#[derive(Debug, Clone, Serialize)]
pub struct Enum {
    pub name: String,
    pub entries: Vec<String>,
}
#[derive(Debug, Clone, Serialize)]
pub struct Property {
    pub name: String,
    pub array_dim: u8,
    pub offset: u16,
    pub inner: PropertyInner,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExtCext {
    pub version: u8,
    pub num_ext: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExtPpth {
    pub version: u8,
    pub enums: Vec<String>,
    pub structs: Vec<String>,
}
#[derive(Debug, Clone, Serialize)]
pub struct ExtEatr {
    pub version: u8,
    pub enum_flags: Vec<u32>,
    pub struct_flags: Vec<StructFlags>,
}

#[derive(Debug, Clone, Serialize)]
pub struct StructFlags {
    pub type_: FlagsType,
    pub value: u32,
    pub prop_flags: Vec<u64>,
}
#[derive(Debug, Clone, Serialize, strum::FromRepr)]
#[repr(u8)]
pub enum FlagsType {
    Unknown,
    Struct,
    Class,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExtEnvp {
    pub version: u8,
    pub value_pairs: Vec<Vec<(String, u64)>>,
}

#[instrument(skip_all)]
pub fn read<R: Read>(reader: &mut R) -> Result<Usmap> {
    read_header(reader)?;
    let names = read_names(reader)?;
    let enums = read_enums(reader, &names)?;
    let structs = read_structs(reader, &names)?;

    let mut cext = None;
    let mut ppth = None;
    let mut eatr = None;
    let mut envp = None;

    loop {
        let mut ext = [0; 4];
        match reader.read_exact(&mut ext) {
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                break;
            }
            r => r,
        }?;
        match &ext {
            b"CEXT" => cext = Some(read_cext(reader)?),
            b"PPTH" => ppth = Some(read_ppth(reader, &names)?),
            b"EATR" => eatr = Some(read_eatr(reader)?),
            b"ENVP" => envp = Some(read_envp(reader, &names)?),
            _ => bail!("ext {ext:X?}"),
        }
    }

    Ok(Usmap {
        //names: &names,
        enums,
        structs,
        cext,
        ppth,
        eatr,
        envp,
    })
}

#[instrument(skip_all)]
fn read_header<R: Read>(reader: &mut R) -> Result<Header> {
    let magic = reader.read_u16::<LE>()?;
    let version = reader.read_u8()?;

    // package versioning

    let _compression_method = reader.read_u8()?;
    let _compressed_size = reader.read_u32::<LE>()?;
    let _decompressed_size = reader.read_u32::<LE>()?;

    Ok(Header { magic, version })
}

#[instrument(skip_all)]
fn read_names<R: Read>(reader: &mut R) -> Result<Vec<String>> {
    let size = reader.read_u32::<LE>()?;
    let mut names = vec![];

    for _ in 0..size {
        names.push(read_string_u8(reader)?);
    }
    Ok(names)
}

#[instrument(skip_all)]
fn read_enums<R: Read>(reader: &mut R, names: &[String]) -> Result<Vec<Enum>> {
    let size = reader.read_u32::<LE>()?;
    let mut enums = vec![];

    for _ in 0..size {
        let name = names[reader.read_u32::<LE>()? as usize].clone();
        let mut entries = vec![];
        for _ in 0..reader.read_u8()? {
            entries.push(names[reader.read_u32::<LE>()? as usize].clone());
        }
        enums.push(Enum { name, entries });
    }
    Ok(enums)
}

#[instrument(skip_all)]
fn read_structs<R: Read>(reader: &mut R, names: &[String]) -> Result<Vec<Struct>> {
    let size = reader.read_u32::<LE>()?;
    let mut structs = vec![];

    for _ in 0..size {
        let name = names[reader.read_u32::<LE>()? as usize].clone();
        let super_struct = reader.read_i32::<LE>()?;
        let super_struct = if super_struct == -1 {
            None
        } else {
            Some(names[super_struct as usize].clone())
        };

        let _prop_count = reader.read_u16::<LE>()?;
        let serializable_prop_count = reader.read_u16::<LE>()?;

        let mut properties = vec![];
        for _ in 0..serializable_prop_count {
            let offset = reader.read_u16::<LE>()?;
            let array_dim = reader.read_u8()?;
            let name = names[reader.read_u32::<LE>()? as usize].clone();
            properties.push(Property {
                array_dim,
                offset,
                name,
                inner: read_property_inner(reader, names)?,
            });
        }
        structs.push(Struct {
            name,
            super_struct,
            properties,
        });
    }
    Ok(structs)
}

#[instrument(skip_all)]
fn read_property_inner<'n, R: Read>(reader: &mut R, names: &'n [String]) -> Result<PropertyInner> {
    let type_ = EPropertyType::from_repr(reader.read_u8()?).context("unknown EPropertyType")?;
    let inner = match type_ {
        EPropertyType::ByteProperty => PropertyInner::Byte,
        EPropertyType::BoolProperty => PropertyInner::Bool,
        EPropertyType::IntProperty => PropertyInner::Int,
        EPropertyType::FloatProperty => PropertyInner::Float,
        EPropertyType::ObjectProperty => PropertyInner::Object,
        EPropertyType::NameProperty => PropertyInner::Name,
        EPropertyType::DelegateProperty => PropertyInner::Delegate,
        EPropertyType::DoubleProperty => PropertyInner::Double,
        EPropertyType::ArrayProperty => PropertyInner::Array {
            inner: Box::new(read_property_inner(reader, names)?),
        },
        EPropertyType::StructProperty => PropertyInner::Struct {
            name: names[reader.read_u32::<LE>()? as usize].clone(),
        },
        EPropertyType::StrProperty => PropertyInner::Str,
        EPropertyType::TextProperty => PropertyInner::Text,
        EPropertyType::InterfaceProperty => PropertyInner::Interface,
        EPropertyType::MulticastDelegateProperty => PropertyInner::MulticastDelegate,
        EPropertyType::WeakObjectProperty => PropertyInner::WeakObject,
        EPropertyType::LazyObjectProperty => PropertyInner::LazyObject,
        EPropertyType::AssetObjectProperty => PropertyInner::AssetObject,
        EPropertyType::SoftObjectProperty => PropertyInner::SoftObject,
        EPropertyType::UInt64Property => PropertyInner::UInt64,
        EPropertyType::UInt32Property => PropertyInner::UInt32,
        EPropertyType::UInt16Property => PropertyInner::UInt16,
        EPropertyType::Int64Property => PropertyInner::Int64,
        EPropertyType::Int16Property => PropertyInner::Int16,
        EPropertyType::Int8Property => PropertyInner::Int8,
        EPropertyType::MapProperty => PropertyInner::Map {
            key: Box::new(read_property_inner(reader, names)?),
            value: Box::new(read_property_inner(reader, names)?),
        },
        EPropertyType::SetProperty => PropertyInner::Set {
            key: Box::new(read_property_inner(reader, names)?),
        },
        EPropertyType::EnumProperty => PropertyInner::Enum {
            inner: Box::new(read_property_inner(reader, names)?),
            name: names[reader.read_u32::<LE>()? as usize].clone(),
            // TODO handle EnumAsByteProperty?
        },
        EPropertyType::FieldPathProperty => PropertyInner::FieldPath,
        EPropertyType::EnumAsByteProperty => todo!("EnumAsByteProperty"),
        EPropertyType::Unknown => todo!("Unknown"),
    };
    Ok(inner)
}

#[instrument(skip_all)]
fn read_cext<R: Read>(reader: &mut R) -> Result<ExtCext> {
    let version = reader.read_u8()?;
    let num_ext = reader.read_u32::<LE>()?;
    Ok(ExtCext { version, num_ext })
}

#[instrument(skip_all)]
fn read_ppth<R: Read>(reader: &mut R, names: &[String]) -> Result<ExtPpth> {
    let _size = reader.read_u32::<LE>()?;
    let version = reader.read_u8()?;
    let mut enums = vec![];
    for _ in 0..reader.read_u32::<LE>()? {
        enums.push(names[reader.read_u32::<LE>()? as usize].clone());
    }
    let mut structs = vec![];
    for _ in 0..reader.read_u32::<LE>()? {
        structs.push(names[reader.read_u32::<LE>()? as usize].clone());
    }
    Ok(ExtPpth {
        version,
        enums,
        structs,
    })
}

#[instrument(skip_all)]
fn read_eatr<R: Read>(reader: &mut R) -> Result<ExtEatr> {
    let _size = reader.read_u32::<LE>()?;
    let version = reader.read_u8()?;
    let mut enum_flags = vec![];
    for _ in 0..reader.read_u32::<LE>()? {
        enum_flags.push(reader.read_u32::<LE>()?);
    }
    let mut struct_flags = vec![];
    for _ in 0..reader.read_u32::<LE>()? {
        let type_ = FlagsType::from_repr(reader.read_u8()?).context("unknown FlagsType")?;
        let value = reader.read_u32::<LE>()?;
        let mut prop_flags = vec![];
        for _ in 0..reader.read_u32::<LE>()? {
            prop_flags.push(reader.read_u64::<LE>()?);
        }
        struct_flags.push(StructFlags {
            type_,
            value,
            prop_flags,
        });
    }
    Ok(ExtEatr {
        version,
        enum_flags,
        struct_flags,
    })
}

#[instrument(skip_all)]
fn read_envp<R: Read>(reader: &mut R, names: &[String]) -> Result<ExtEnvp> {
    let _size = reader.read_u32::<LE>()?;
    let version = reader.read_u8()?;
    let mut value_pairs = vec![];
    for _ in 0..reader.read_u32::<LE>()? {
        let mut n = vec![];
        for _ in 0..reader.read_u32::<LE>()? {
            n.push((
                names[reader.read_u32::<LE>()? as usize].clone(),
                reader.read_u64::<LE>()?,
            ));
        }
        value_pairs.push(n);
    }
    Ok(ExtEnvp {
        version,
        value_pairs,
    })
}

#[instrument(skip_all)]
fn read_cstr<R: Read>(reader: &mut R) -> Result<String> {
    let mut buf = vec![];
    loop {
        let next = reader.read_u8()?;
        if next == 0 {
            break;
        }
        buf.push(next);
    }
    Ok(String::from_utf8(buf)?)
}

#[instrument(skip_all)]
fn read_string_u8<R: Read>(reader: &mut R) -> Result<String> {
    let length = reader.read_u8()?;
    let mut buf = vec![0; length as usize];
    reader.read_exact(&mut buf)?;
    Ok(String::from_utf8(
        buf.into_iter().take_while(|b| *b != 0).collect::<Vec<_>>(),
    )?)
}

#[cfg(test)]
mod test {
    use super::*;

    fn test_usmap(path: &str) -> Result<()> {
        let mut input = std::io::Cursor::new(std::fs::read(path)?);
        let res = ser_hex::read("trace.json", &mut input, read)?;
        println!("{res:#?}");
        Ok(())
    }
    #[test]
    fn test_4_27() -> Result<()> {
        test_usmap("tests/drg.usmap")
    }
    #[test]
    fn test_5_4() -> Result<()> {
        test_usmap("tests/5.4.3-34507850+++UE5+Release-5.4-DeepSpace7.usmap")
    }
}

