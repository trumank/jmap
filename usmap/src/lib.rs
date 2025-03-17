mod compression;
mod gen;

use std::{
    collections::HashMap,
    io::{Read, Seek, Write},
};

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
struct SerCtx<'c, S> {
    inner: S,
    header: &'c Header,
    names: &'c mut Names,
}
impl<'c, S> Read for SerCtx<'c, S>
where
    S: Read,
{
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.inner.read(buf)
    }
}
impl<'c, S> Write for SerCtx<'c, S>
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
impl<'c, S> Seek for SerCtx<'c, S>
where
    S: Seek,
{
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        self.inner.seek(pos)
    }
}
impl<'c, S> SerCtx<'c, S> {
    fn new(inner: S, header: &'c Header, names: &'c mut Names) -> Self {
        Self {
            inner,
            header,
            names,
        }
    }
    fn chain<I>(&mut self, inner: I) -> SerCtx<I> {
        SerCtx {
            inner,
            header: self.header,
            names: self.names,
        }
    }
}
impl<'c, S: Read> SerCtx<'c, S> {
    #[instrument(skip_all)]
    fn read_names(&mut self) -> Result<()> {
        for _ in 0..self.read_u32::<LE>()? {
            let length = if self.header.version >= UsmapVersion::LongFName {
                self.read_u16::<LE>()? as usize
            } else {
                self.read_u8()? as usize
            };
            let mut buf = vec![0; length];
            self.read_exact(&mut buf)?;
            let name =
                String::from_utf8(buf.into_iter().take_while(|b| *b != 0).collect::<Vec<_>>())?;
            self.names.insert_dup(name);
        }
        Ok(())
    }
    fn read_name(&mut self) -> Result<String> {
        let i = self.inner.read_u32::<LE>()?;
        Ok(self.names.get(i).to_string())
    }
    fn read_opt_name(&mut self) -> Result<Option<String>> {
        let i = self.inner.read_u32::<LE>()?;
        Ok((i != u32::MAX).then(|| self.names.get(i).to_string()))
    }
}
impl<S: Write> SerCtx<'_, S> {
    #[instrument(skip_all)]
    fn write_names(&mut self) -> Result<()> {
        self.write_u32::<LE>(self.names.names.len() as u32)?;
        // split mutable borrow curse
        let (s, names) = (&mut self.inner, &self.names);
        for name in &names.names {
            if self.header.version >= UsmapVersion::LongFName {
                s.write_u16::<LE>(name.len().try_into().expect("name too long"))?;
            } else {
                s.write_u8(name.len().try_into().expect("name too long"))?;
            };
            s.write_all(name.as_bytes())?;
        }
        Ok(())
    }
    fn write_name(&mut self, name: String) -> Result<()> {
        let i = self.names.insert(name);
        self.write_u32::<LE>(i)?;
        Ok(())
    }
    fn write_opt_name(&mut self, name: Option<String>) -> Result<()> {
        let i = if let Some(name) = name {
            self.names.insert(name)
        } else {
            u32::MAX
        };
        self.write_u32::<LE>(i)?;
        Ok(())
    }
}

#[derive(Default)]
struct Names {
    names: Vec<String>,
    index: HashMap<String, u32>,
}
impl Names {
    fn new() -> Self {
        Self::default()
    }
    fn get(&self, index: u32) -> &str {
        &self.names[index as usize]
    }
    fn insert(&mut self, name: String) -> u32 {
        match self.index.entry(name) {
            std::collections::hash_map::Entry::Occupied(entry) => *entry.get(),
            std::collections::hash_map::Entry::Vacant(entry) => {
                let index = self.names.len() as u32;
                self.names.push(entry.key().clone());
                entry.insert(index);
                index
            }
        }
    }
    // needed because *some* usmap generators have duplicates in name map
    fn insert_dup(&mut self, name: String) -> u32 {
        let index = self.names.len() as u32;
        self.names.push(name.clone());
        self.index.insert(name, index);
        index
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
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
impl PropertyInner {
    fn get_type(&self) -> EPropertyType {
        match self {
            PropertyInner::Byte => EPropertyType::ByteProperty,
            PropertyInner::Bool => EPropertyType::BoolProperty,
            PropertyInner::Int => EPropertyType::IntProperty,
            PropertyInner::Float => EPropertyType::FloatProperty,
            PropertyInner::Object => EPropertyType::ObjectProperty,
            PropertyInner::Name => EPropertyType::NameProperty,
            PropertyInner::Delegate => EPropertyType::DelegateProperty,
            PropertyInner::Double => EPropertyType::DoubleProperty,
            PropertyInner::Array { .. } => EPropertyType::ArrayProperty,
            PropertyInner::Struct { .. } => EPropertyType::StructProperty,
            PropertyInner::Str => EPropertyType::StrProperty,
            PropertyInner::Text => EPropertyType::TextProperty,
            PropertyInner::Interface => EPropertyType::InterfaceProperty,
            PropertyInner::MulticastDelegate => EPropertyType::MulticastDelegateProperty,
            PropertyInner::WeakObject => EPropertyType::WeakObjectProperty,
            PropertyInner::LazyObject => EPropertyType::LazyObjectProperty,
            PropertyInner::AssetObject => EPropertyType::AssetObjectProperty,
            PropertyInner::SoftObject => EPropertyType::SoftObjectProperty,
            PropertyInner::UInt64 => EPropertyType::UInt64Property,
            PropertyInner::UInt32 => EPropertyType::UInt32Property,
            PropertyInner::UInt16 => EPropertyType::UInt16Property,
            PropertyInner::Int64 => EPropertyType::Int64Property,
            PropertyInner::Int16 => EPropertyType::Int16Property,
            PropertyInner::Int8 => EPropertyType::Int8Property,
            PropertyInner::Map { .. } => EPropertyType::MapProperty,
            PropertyInner::Set { .. } => EPropertyType::SetProperty,
            PropertyInner::Enum { .. } => EPropertyType::EnumProperty,
            PropertyInner::FieldPath => EPropertyType::FieldPathProperty,
            PropertyInner::Optional { .. } => EPropertyType::OptionalProperty,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Usmap {
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
    pub version: UsmapVersion,

    pub compression_method: Option<CompressionMethod>,
    pub compressed_size: u32,
    pub decompressed_size: u32,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Struct {
    pub name: String,
    pub super_struct: Option<String>,
    pub properties: Vec<Property>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Enum {
    pub name: String,
    pub entries: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Property {
    pub name: String,
    pub array_dim: u8,
    pub offset: u16,
    pub inner: PropertyInner,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ExtCext {
    pub version: u8,
    pub num_ext: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ExtPpth {
    pub version: u8,
    pub enums: Vec<String>,
    pub structs: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ExtEatr {
    pub version: u8,
    pub enum_flags: Vec<u32>,
    pub struct_flags: Vec<StructFlags>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StructFlags {
    pub type_: FlagsType,
    pub value: u32,
    pub prop_flags: Vec<u64>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, strum::FromRepr)]
#[repr(u8)]
pub enum FlagsType {
    Unknown,
    Struct,
    Class,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
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
        let mut names = Names::new();
        let s = &mut SerCtx::new(s, &header, &mut names);

        s.read_names()?;
        let enums = read_enums(s)?;
        let structs = read_structs(s)?;

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
                b"CEXT" => cext = Some(ExtCext::read(s)?),
                b"PPTH" => ppth = Some(ExtPpth::read(s)?),
                b"EATR" => eatr = Some(ExtEatr::read(s)?),
                b"ENVP" => envp = Some(ExtEnvp::read(s)?),
                _ => bail!("ext {ext:X?}"),
            }
        }

        Ok(Usmap {
            enums,
            structs,
            cext,
            ppth,
            eatr,
            envp,
        })
    }
    #[instrument(skip_all, name = "Usmap::write")]
    pub fn write<S: Write>(&self, s: &mut S) -> Result<()> {
        let mut names = Names::new();
        let header = Header {
            version: UsmapVersion::PackageVersioning,
            compression_method: None,
            compressed_size: 0,
            decompressed_size: 0,
        };
        let mut full_buffer = vec![];
        {
            let s = &mut SerCtx::new(&mut full_buffer, &header, &mut names);

            let mut buffer: Vec<u8> = vec![];
            {
                let s = &mut s.chain(&mut buffer);
                write_enums(s, &self.enums)?;
                write_structs(s, &self.structs)?;

                if let Some(ext) = &self.cext {
                    s.write_all(b"CEXT")?;
                    ext.write(s)?;
                }
                if let Some(ext) = &self.ppth {
                    s.write_all(b"PPTH")?;
                    ext.write(s)?;
                }
                if let Some(ext) = &self.eatr {
                    s.write_all(b"EATR")?;
                    ext.write(s)?;
                }
                if let Some(ext) = &self.envp {
                    s.write_all(b"ENVP")?;
                    ext.write(s)?;
                }
            }
            s.write_names()?;
            s.write_all(&buffer)?;
        }

        header.write(s)?;
        s.write_all(&full_buffer)?;

        Ok(())
    }
}

impl Header {
    const MAGIC: u16 = 0x30C4;
    #[instrument(skip_all, name = "Header::read")]
    fn read<S: Read>(s: &mut S) -> Result<Self> {
        let magic = s.read_u16::<LE>()?;
        if magic != Self::MAGIC {
            bail!("bad Usmap magic {magic:04x}");
        }
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
            version,
            compression_method,
            compressed_size,
            decompressed_size,
        })
    }
    #[instrument(skip_all, name = "Header::read")]
    fn write<S: Write>(&self, s: &mut S) -> Result<()> {
        s.write_u16::<LE>(Self::MAGIC)?;
        self.version.write(s)?;

        if self.version >= UsmapVersion::PackageVersioning {
            s.write_i32::<LE>(0)?;
            // TODO versioning
        }

        self.compression_method.write(s)?;
        s.write_u32::<LE>(self.compressed_size)?;
        s.write_u32::<LE>(self.decompressed_size)?;

        Ok(())
    }
}

#[instrument(skip_all)]
fn read_enums<S: Read>(s: &mut SerCtx<S>) -> Result<Vec<Enum>> {
    let size = s.read_u32::<LE>()?;
    let mut enums = vec![];

    for _ in 0..size {
        let name = s.read_name()?;
        let mut entries = vec![];
        let num_entries = if s.header.version >= UsmapVersion::LargeEnums {
            s.read_u16::<LE>()? as usize
        } else {
            s.read_u8()? as usize
        };
        for _ in 0..num_entries {
            entries.push(s.read_name()?);
        }
        enums.push(Enum { name, entries });
    }
    Ok(enums)
}
#[instrument(skip_all)]
fn write_enums<S: Write>(s: &mut SerCtx<S>, enums: &[Enum]) -> Result<()> {
    s.write_u32::<LE>(enums.len() as u32)?;

    for e in enums {
        s.write_name(e.name.clone())?;
        if s.header.version >= UsmapVersion::LargeEnums {
            s.write_u16::<LE>(e.entries.len().try_into().expect("enum entries too large"))?;
        } else {
            s.write_u8(e.entries.len().try_into().expect("enum entries too large"))?;
        }
        for entry in &e.entries {
            s.write_name(entry.clone())?;
        }
    }
    Ok(())
}

#[instrument(skip_all)]
fn read_structs<S: Read>(s: &mut SerCtx<S>) -> Result<Vec<Struct>> {
    let size = s.read_u32::<LE>()?;
    let mut structs = vec![];

    for _ in 0..size {
        let name = s.read_name()?;
        let super_struct = s.read_opt_name()?;

        let _prop_count = s.read_u16::<LE>()?;
        let serializable_prop_count = s.read_u16::<LE>()?;

        let mut properties = vec![];
        for _ in 0..serializable_prop_count {
            let offset = s.read_u16::<LE>()?;
            let array_dim = s.read_u8()?;
            let name = s.read_name()?;
            properties.push(Property {
                array_dim,
                offset,
                name,
                inner: read_property_inner(s)?,
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
fn write_structs<S: Write>(s: &mut SerCtx<S>, structs: &[Struct]) -> Result<()> {
    s.write_u32::<LE>(structs.len() as u32)?;
    for struct_ in structs {
        s.write_name(struct_.name.clone())?;
        s.write_opt_name(struct_.super_struct.clone())?;

        // TODO when does prop_count != serializable_prop_count?
        s.write_u16::<LE>(struct_.properties.len().try_into().unwrap())?;
        s.write_u16::<LE>(struct_.properties.len().try_into().unwrap())?;

        for prop in &struct_.properties {
            s.write_u16::<LE>(prop.offset)?;
            s.write_u8(prop.array_dim)?;
            s.write_name(prop.name.clone())?;
            write_property_inner(s, &prop.inner)?;
        }
    }
    Ok(())
}

#[instrument(skip_all)]
fn read_property_inner<S: Read>(s: &mut SerCtx<S>) -> Result<PropertyInner> {
    let type_ = EPropertyType::from_repr(s.read_u8()?).context("unknown EPropertyType")?;
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
            inner: read_property_inner(s)?.into(),
        },
        EPropertyType::StructProperty => PropertyInner::Struct {
            name: s.read_name()?,
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
            key: read_property_inner(s)?.into(),
            value: read_property_inner(s)?.into(),
        },
        EPropertyType::SetProperty => PropertyInner::Set {
            key: read_property_inner(s)?.into(),
        },
        EPropertyType::EnumProperty => PropertyInner::Enum {
            inner: read_property_inner(s)?.into(),
            name: s.read_name()?,
            // TODO handle EnumAsByteProperty?
        },
        EPropertyType::FieldPathProperty => PropertyInner::FieldPath,
        EPropertyType::OptionalProperty => PropertyInner::Optional {
            inner: read_property_inner(s)?.into(),
        },
        EPropertyType::Unknown => todo!("Unknown"),
    };
    Ok(inner)
}

#[instrument(skip_all)]
fn write_property_inner<S: Write>(s: &mut SerCtx<S>, prop: &PropertyInner) -> Result<()> {
    s.write_u8(prop.get_type() as u8)?;
    match prop {
        PropertyInner::Array { inner } => {
            write_property_inner(s, inner)?;
        }
        PropertyInner::Struct { name } => {
            s.write_name(name.clone())?;
        }
        PropertyInner::Map { key, value } => {
            write_property_inner(s, key)?;
            write_property_inner(s, value)?;
        }
        PropertyInner::Set { key } => {
            write_property_inner(s, key)?;
        }
        PropertyInner::Enum { inner, name } => {
            write_property_inner(s, inner)?;
            s.write_name(name.clone())?;
        }
        PropertyInner::Optional { inner } => {
            write_property_inner(s, inner)?;
        }
        _ => {}
    }
    Ok(())
}

impl ExtCext {
    #[instrument(skip_all, name = "ExtCext::read")]
    fn read<S: Read>(s: &mut SerCtx<S>) -> Result<Self> {
        let version = s.read_u8()?;
        let num_ext = s.read_u32::<LE>()?;
        Ok(Self { version, num_ext })
    }
    #[instrument(skip_all, name = "ExtCext::write")]
    fn write<S: Write>(&self, s: &mut SerCtx<S>) -> Result<()> {
        s.write_u8(self.version)?;
        s.write_u32::<LE>(self.num_ext)?;
        Ok(())
    }
}

impl ExtPpth {
    #[instrument(skip_all, name = "ExpPpth::read")]
    fn read<S: Read>(s: &mut SerCtx<S>) -> Result<Self> {
        let _size = s.read_u32::<LE>()?;
        let version = s.read_u8()?;
        let mut enums = vec![];
        for _ in 0..s.read_u32::<LE>()? {
            enums.push(s.read_name()?);
        }
        let mut structs = vec![];
        for _ in 0..s.read_u32::<LE>()? {
            structs.push(s.read_name()?);
        }
        Ok(Self {
            version,
            enums,
            structs,
        })
    }
    #[instrument(skip_all, name = "ExpPpth::write")]
    fn write<S: Write>(&self, s: &mut SerCtx<S>) -> Result<()> {
        let mut buffer = vec![];
        {
            let s = &mut s.chain(&mut buffer);
            s.write_u8(self.version)?;
            s.write_u32::<LE>(self.enums.len() as u32)?;
            for enum_ in &self.enums {
                s.write_name(enum_.clone())?;
            }
            s.write_u32::<LE>(self.structs.len() as u32)?;
            for struct_ in &self.structs {
                s.write_name(struct_.clone())?;
            }
        }
        s.write_u32::<LE>(buffer.len() as u32)?;
        s.write_all(&buffer)?;
        Ok(())
    }
}

impl ExtEatr {
    #[instrument(skip_all, name = "ExtEatr::read")]
    fn read<S: Read>(s: &mut SerCtx<S>) -> Result<Self> {
        let _size = s.read_u32::<LE>()?;
        let version = s.read_u8()?;
        let mut enum_flags = vec![];
        for _ in 0..s.read_u32::<LE>()? {
            enum_flags.push(s.read_u32::<LE>()?);
        }
        let mut struct_flags = vec![];
        for _ in 0..s.read_u32::<LE>()? {
            let type_ = FlagsType::from_repr(s.read_u8()?).context("unknown FlagsType")?;
            let value = s.read_u32::<LE>()?;
            let mut prop_flags = vec![];
            for _ in 0..s.read_u32::<LE>()? {
                prop_flags.push(s.read_u64::<LE>()?);
            }
            struct_flags.push(StructFlags {
                type_,
                value,
                prop_flags,
            });
        }
        Ok(Self {
            version,
            enum_flags,
            struct_flags,
        })
    }
    #[instrument(skip_all, name = "ExpEatr::write")]
    fn write<S: Write>(&self, s: &mut SerCtx<S>) -> Result<()> {
        let mut buffer = vec![];
        {
            let s = &mut s.chain(&mut buffer);
            s.write_u8(self.version)?;

            s.write_u32::<LE>(self.enum_flags.len() as u32)?;
            for flags in &self.enum_flags {
                s.write_u32::<LE>(*flags)?;
            }
            s.write_u32::<LE>(self.struct_flags.len() as u32)?;
            for flags in &self.struct_flags {
                s.write_u8(flags.type_ as u8)?;
                s.write_u32::<LE>(flags.value)?;

                s.write_u32::<LE>(flags.prop_flags.len() as u32)?;
                for flags in &flags.prop_flags {
                    s.write_u64::<LE>(*flags)?;
                }
            }
        }
        s.write_u32::<LE>(buffer.len() as u32)?;
        s.write_all(&buffer)?;
        Ok(())
    }
}

impl ExtEnvp {
    #[instrument(skip_all, name = "ExtEnvp::read")]
    fn read<S: Read>(s: &mut SerCtx<S>) -> Result<Self> {
        let _size = s.read_u32::<LE>()?;
        let version = s.read_u8()?;
        let mut value_pairs = vec![];
        for _ in 0..s.read_u32::<LE>()? {
            let mut n = vec![];
            for _ in 0..s.read_u32::<LE>()? {
                n.push((s.read_name()?, s.read_u64::<LE>()?));
            }
            value_pairs.push(n);
        }
        Ok(Self {
            version,
            value_pairs,
        })
    }
    #[instrument(skip_all, name = "ExpEnvp::write")]
    fn write<S: Write>(&self, s: &mut SerCtx<S>) -> Result<()> {
        let mut buffer = vec![];
        {
            let s = &mut s.chain(&mut buffer);
            s.write_u8(self.version)?;

            s.write_u32::<LE>(self.value_pairs.len() as u32)?;
            for pairs in &self.value_pairs {
                s.write_u32::<LE>(pairs.len() as u32)?;
                for (name, value) in pairs {
                    s.write_name(name.clone())?;
                    s.write_u64::<LE>(*value)?;
                }
            }
        }
        s.write_u32::<LE>(buffer.len() as u32)?;
        s.write_all(&buffer)?;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn test_usmap(path: &str) -> Result<()> {
        let mut input = std::io::Cursor::new(std::fs::read(path)?);
        let res = ser_hex::read("trace.json", &mut input, Usmap::read)?;

        let mut buffer = vec![];
        res.write(&mut buffer)?;

        let mut input = std::io::Cursor::new(buffer);
        let res2 = ser_hex::read("trace_rt.json", &mut input, Usmap::read)?;
        assert_eq!(res, res2);

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
