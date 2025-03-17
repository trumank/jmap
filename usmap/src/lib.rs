mod compression;
mod gen;

use std::io::{Read, Seek, Write};

use anyhow::{bail, Context, Result};

use byteorder::{ReadBytesExt, WriteBytesExt, LE};
use serde::Serialize;
use tracing::instrument;

trait Ser {
    fn read<S: Read>(s: &mut S) -> Result<Self>
    where
        Self: Sized;
    fn write<S: Write>(&self, s: &mut S) -> Result<()>;
}
struct SerCtx<S> {
    inner: S,
    header: Header,
}
impl<S> Read for SerCtx<S>
where
    S: Read,
{
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.inner.read(buf)
    }
}
impl<S> Write for SerCtx<S>
where
    S: Write,
{
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.inner.write(buf)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}
impl<S> Seek for SerCtx<S>
where
    S: Seek,
{
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        self.inner.seek(pos)
    }
}

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
    OptionalProperty,

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
    Optional {
        inner: Box<PropertyInner>,
    },
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, strum::FromRepr)]
#[repr(u8)]
pub enum UsmapVersion {
    Initial,
    PackageVersioning,
    LongFName,
    LargeEnums,
}
impl UsmapVersion {
    #[instrument(skip_all, name = "UsmapVersion::read")]
    pub fn read<S: Read>(s: &mut S) -> Result<Self> {
        let v = s.read_u8()?;
        Self::from_repr(v).with_context(|| format!("Unrecognized version {v}"))
    }
    pub fn write<S: Write>(&self, s: &mut S) -> Result<()> {
        Ok(s.write_u8(*self as u8)?)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, strum::FromRepr)]
#[repr(u8)]
pub enum CompressionMethod {
    Oodle = 1,
    Brotli = 2,
    Zstd = 3,
}
impl Ser for Option<CompressionMethod> {
    #[instrument(skip_all, name = "Option<CompressionMethod>::read")]
    fn read<S: Read>(s: &mut S) -> Result<Self> {
        let v = s.read_u8()?;
        Ok(if v == 0 {
            None
        } else {
            Some(
                CompressionMethod::from_repr(v)
                    .with_context(|| format!("Unknown compression method {v}"))?,
            )
        })
    }
    fn write<S: Write>(&self, s: &mut S) -> Result<()> {
        Ok(s.write_u8(self.map_or(0, |m| m as u8))?)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Header {
    pub magic: u16,
    pub version: UsmapVersion,

    pub compression_method: Option<CompressionMethod>,
    pub compressed_size: u32,
    pub decompressed_size: u32,
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

impl Usmap {
    #[instrument(skip_all, name = "Usmap::read")]
    pub fn read<S: Read>(s: &mut S) -> Result<Usmap> {
        let header = Header::read(s)?;
        dbg!(&header);

        let mut rest = vec![];
        s.read_to_end(&mut rest)?;
        let buffer = match header.compression_method {
            None => rest,
            Some(m) => {
                let mut out = vec![0; header.decompressed_size as usize];
                compression::decompress(m, &rest, &mut out)?;
                out
            }
        };

        let s = &mut ser_hex::TraceStream::new("trace_inner.json", std::io::Cursor::new(buffer));
        let s = &mut SerCtx { inner: s, header };

        let names = read_names(s)?;
        let enums = read_enums(s, &names)?;
        let structs = read_structs(s, &names)?;

        let mut cext = None;
        let mut ppth = None;
        let mut eatr = None;
        let mut envp = None;

        loop {
            let mut ext = [0; 4];
            match s.read_exact(&mut ext) {
                Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                    break;
                }
                r => r,
            }?;
            match &ext {
                b"CEXT" => cext = Some(read_cext(s)?),
                b"PPTH" => ppth = Some(read_ppth(s, &names)?),
                b"EATR" => eatr = Some(read_eatr(s)?),
                b"ENVP" => envp = Some(read_envp(s, &names)?),
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
}

impl Header {
    #[instrument(skip_all, name = "Header::read")]
    fn read<S: Read>(s: &mut S) -> Result<Self> {
        let magic = s.read_u16::<LE>()?;
        let version = UsmapVersion::read(s)?;

        // package versioning

        if version >= UsmapVersion::PackageVersioning {
            let has_versioning = s.read_i32::<LE>()? > 0;
            if has_versioning {
                // TODO import UE version enums
                let file_version_ue4 = s.read_i32::<LE>()?;
                let file_version_ue5 = s.read_i32::<LE>()?;

                let mut custom_version_container = vec![];
                for _ in 0..s.read_u32::<LE>()? {
                    let mut guid = [0; 20];
                    s.read_exact(&mut guid)?;
                    let version_number = s.read_i32::<LE>()?;
                    custom_version_container.push((guid, version_number));
                }
                let net_cl = s.read_i32::<LE>()?;
            }
        }

        let compression_method = Option::<CompressionMethod>::read(s)?;
        let compressed_size = s.read_u32::<LE>()?;
        let decompressed_size = s.read_u32::<LE>()?;

        Ok(Self {
            magic,
            version,
            compression_method,
            compressed_size,
            decompressed_size,
        })
    }
}

#[instrument(skip_all)]
fn read_names<S: Read>(s: &mut SerCtx<S>) -> Result<Vec<String>> {
    let size = s.read_u32::<LE>()?;
    let mut names = vec![];

    for _ in 0..size {
        names.push(read_string_u8(s)?);
    }
    Ok(names)
}

#[instrument(skip_all)]
fn read_enums<S: Read>(s: &mut SerCtx<S>, names: &[String]) -> Result<Vec<Enum>> {
    let size = s.read_u32::<LE>()?;
    let mut enums = vec![];

    for _ in 0..size {
        let name = names[s.read_u32::<LE>()? as usize].clone();
        let mut entries = vec![];
        let num_entries = if s.header.version >= UsmapVersion::LargeEnums {
            s.read_u16::<LE>()? as usize
        } else {
            s.read_u8()? as usize
        };
        for _ in 0..num_entries {
            entries.push(names[s.read_u32::<LE>()? as usize].clone());
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
        EPropertyType::OptionalProperty => PropertyInner::Optional {
            inner: Box::new(read_property_inner(reader, names)?),
        },
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
fn read_string_u8<S: Read>(s: &mut SerCtx<S>) -> Result<String> {
    let length = if s.header.version >= UsmapVersion::LongFName {
        s.read_u16::<LE>()? as usize
    } else {
        s.read_u8()? as usize
    };
    let mut buf = vec![0; length];
    s.read_exact(&mut buf)?;
    Ok(String::from_utf8(
        buf.into_iter().take_while(|b| *b != 0).collect::<Vec<_>>(),
    )?)
}

#[cfg(test)]
mod test {
    use super::*;

    fn test_usmap(path: &str) -> Result<()> {
        let mut input = std::io::Cursor::new(std::fs::read(path)?);
        let res = ser_hex::read("trace.json", &mut input, Usmap::read)?;
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
