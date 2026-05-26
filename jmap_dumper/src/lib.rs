pub mod containers;
mod header;
pub mod mem;
pub mod objects;
mod proc_name;
pub mod structs;
mod vtable;

pub use header::into_header;

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result, bail};
use containers::{FName, FScriptMap, FScriptSet, FString};
use futures_util::StreamExt;
use jmap::{
    BytePropertyValue, Class, EClassCastFlags, EObjectFlags, EngineVersion, Enum,
    EnumPropertyValue, Function, Jmap, Metadata, Object, ObjectType, Package, Property,
    PropertyType, PropertyValue, ScriptStruct, Struct,
};
use mem::{BlockCache, Ctx, MachoCoreMem, ProcessHandle, Ptr};
use objects::FOptionalProperty;
use ordermap::OrderMap;
use patternsleuth::image::Image;
use patternsleuth::resolvers::{impl_collector, resolve};

use crate::containers::{FUtf8String, extract_fnames};
use crate::objects::{
    FUObjectArray, UClass, UEnum, UFunction, UObject, UScriptStruct, UStruct, ZArrayProperty,
    ZBoolProperty, ZByteProperty, ZClassProperty, ZDelegateProperty, ZEnumProperty,
    ZInterfaceProperty, ZLazyObjectProperty, ZMapProperty, ZMulticastDelegateProperty,
    ZObjectProperty, ZProperty, ZSetProperty, ZSoftClassProperty, ZSoftObjectProperty,
    ZStructProperty, ZWeakObjectProperty,
};
use crate::structs::Structs;

impl_collector! {
    #[derive(Debug, PartialEq, Clone)]
    struct Resolution {
        guobject_array: patternsleuth::resolvers::unreal::guobject_array::GUObjectArray,
        fname_pool: patternsleuth::resolvers::unreal::fname::FNamePool,
        engine_version: patternsleuth::resolvers::unreal::engine_version::EngineVersion,
        build: patternsleuth::resolvers::unreal::engine_version::BuildChangeList,
        fname_constant: patternsleuth::resolvers::unreal::fname::StaticFNameConst,
    }
}

async fn read_path(obj: &Ptr<UObject>) -> Result<String> {
    let mut objects = vec![obj.clone()];

    let mut obj = obj.clone();
    while let Some(outer) = obj.outer_private().read().await? {
        objects.push(outer.clone());
        obj = outer;
    }

    let mut path = String::new();
    let mut prev: Option<&Ptr<UObject>> = None;
    for obj in objects.iter().rev() {
        if let Some(prev) = prev {
            let sep = if prev
                .class_private()
                .read()
                .await?
                .class_cast_flags()
                .read()
                .await?
                .contains(EClassCastFlags::CASTCLASS_UPackage)
            {
                '.'
            } else {
                ':'
            };
            path.push(sep);
        }
        path.push_str(&obj.name_private().read().await?);
        prev = Some(obj);
    }

    Ok(path)
}

#[derive(Clone)]
pub struct MemoryRegion<'a> {
    base_address: u64,
    end_address: u64,
    data: &'a [u8],
}

#[derive(Clone)]
pub struct MinidumpMem<'a> {
    regions: Arc<Vec<MemoryRegion<'a>>>,
}

impl<'a> MinidumpMem<'a> {
    pub fn new(minidump: &'a minidump::Minidump<'_, &'a [u8]>) -> Result<Self> {
        use minidump::UnifiedMemory;

        let mut regions = Vec::new();

        let memory_list = minidump
            .get_memory()
            .context("No memory list in minidump")?;

        for memory_region in memory_list.iter() {
            let (base_address, bytes) = match memory_region {
                UnifiedMemory::Memory(mem) => (mem.base_address, mem.bytes),
                UnifiedMemory::Memory64(mem) => (mem.base_address, mem.bytes),
            };

            if !bytes.is_empty() {
                let end_address = base_address + bytes.len() as u64;
                regions.push(MemoryRegion {
                    base_address,
                    end_address,
                    data: bytes,
                });
            }
        }

        regions.sort_by_key(|r| r.base_address);

        Ok(MinidumpMem {
            regions: Arc::new(regions),
        })
    }
}

pub struct OpenMinidump {
    pub minidump: &'static minidump::Minidump<'static, &'static [u8]>,
    pub image: patternsleuth::image::Image<'static>,
}

/// Load and leak a minidump to `'static`, *without* building a patternsleuth
pub fn load_minidump(
    path: impl AsRef<std::path::Path>,
) -> Result<&'static minidump::Minidump<'static, &'static [u8]>> {
    let file = std::fs::File::open(path)?;
    let mmap: &'static memmap2::Mmap =
        Box::leak(Box::new(unsafe { memmap2::MmapOptions::new().map(&file)? }));
    let minidump: &'static minidump::Minidump<'static, &'static [u8]> =
        Box::leak(Box::new(minidump::Minidump::read(&**mmap)?));
    Ok(minidump)
}

/// Open a minidump file, leaking the mmap and parsed minidump to get `'static`
/// lifetimes, and build a patternsleuth image for resolution.
pub fn open_minidump(path: impl AsRef<std::path::Path>) -> Result<OpenMinidump> {
    let minidump = load_minidump(path)?;
    let image = patternsleuth::image::pe::read_image_from_minidump(minidump)?;
    Ok(OpenMinidump { minidump, image })
}

#[async_trait::async_trait(?Send)]
impl mem::Mem for MinidumpMem<'_> {
    async fn read_buf(&self, address: u64, buf: &mut [u8]) -> Result<()> {
        let mut bytes_read = 0;
        let total_bytes = buf.len();
        let read_end_address = address + total_bytes as u64;

        let start_idx = self
            .regions
            .binary_search_by(|region| {
                if region.end_address <= address {
                    std::cmp::Ordering::Less
                } else if region.base_address > address {
                    std::cmp::Ordering::Greater
                } else {
                    std::cmp::Ordering::Equal
                }
            })
            .unwrap_or_else(|i| i);

        for region in &self.regions[start_idx..] {
            if bytes_read >= total_bytes {
                break;
            }

            if region.base_address >= read_end_address {
                break;
            }

            if address < region.end_address && region.base_address < read_end_address {
                let read_start = address.max(region.base_address);
                let read_end = read_end_address.min(region.end_address);

                if read_start < read_end {
                    let region_offset = (read_start - region.base_address) as usize;
                    let buf_offset = (read_start - address) as usize;
                    let copy_len = (read_end - read_start) as usize;

                    buf[buf_offset..buf_offset + copy_len]
                        .copy_from_slice(&region.data[region_offset..region_offset + copy_len]);

                    bytes_read += copy_len;
                }
            }
        }

        if bytes_read < total_bytes {
            bail!(
                "Only read {}/{} bytes starting at address 0x{:x}",
                bytes_read,
                total_bytes,
                address
            );
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
struct ChunkEntry {
    address: u64,
    offset: u64,
    length: u64,
}

/// Dead simple memory dump layout. Concatenated memory blocks with address mapping index at the end.
#[derive(Clone)]
pub struct ConcatMem {
    data: Arc<memmap2::Mmap>,
    chunks: Arc<[ChunkEntry]>,
}

impl ConcatMem {
    pub fn open(path: impl AsRef<std::path::Path>) -> Result<Self> {
        let file = std::fs::File::open(path)?;
        let mmap = unsafe { memmap2::MmapOptions::new().map(&file)? };
        Self::new(Arc::new(mmap))
    }

    fn new(data: Arc<memmap2::Mmap>) -> Result<Self> {
        if data.len() < 8 {
            bail!("File too small");
        }

        let index_offset = u64::from_le_bytes(data[data.len() - 8..].try_into().unwrap()) as usize;

        if index_offset + 8 > data.len() {
            bail!("Invalid index offset");
        }

        let num_chunks =
            u64::from_le_bytes(data[index_offset..index_offset + 8].try_into().unwrap()) as usize;

        let mut chunks = Vec::with_capacity(num_chunks);
        let mut pos = index_offset + 8;

        for _ in 0..num_chunks {
            if pos + 24 > data.len() {
                bail!("Invalid chunk entry");
            }

            let address = u64::from_le_bytes(data[pos..pos + 8].try_into().unwrap());
            let offset = u64::from_le_bytes(data[pos + 8..pos + 16].try_into().unwrap());
            let length = u64::from_le_bytes(data[pos + 16..pos + 24].try_into().unwrap());

            chunks.push(ChunkEntry {
                address,
                offset,
                length,
            });
            pos += 24;
        }

        chunks.sort_by_key(|chunk| chunk.address);

        Ok(Self {
            data,
            chunks: chunks.into(),
        })
    }

    fn find_chunk(&self, address: u64) -> Option<&ChunkEntry> {
        self.chunks
            .binary_search_by(|chunk| {
                if address < chunk.address {
                    std::cmp::Ordering::Greater
                } else if address >= chunk.address + chunk.length {
                    std::cmp::Ordering::Less
                } else {
                    std::cmp::Ordering::Equal
                }
            })
            .ok()
            .map(|idx| &self.chunks[idx])
    }
}

#[async_trait::async_trait(?Send)]
impl mem::Mem for ConcatMem {
    async fn read_buf(&self, address: u64, buf: &mut [u8]) -> Result<()> {
        let chunk = self
            .find_chunk(address)
            .ok_or_else(|| anyhow::anyhow!("Address {:#x} not found in any chunk", address))?;

        let offset_in_chunk = (address - chunk.address) as usize;
        let file_offset = chunk.offset as usize + offset_in_chunk;

        if file_offset + buf.len() > self.data.len() {
            bail!("Read beyond file bounds");
        }

        if offset_in_chunk + buf.len() > chunk.length as usize {
            bail!("Read beyond chunk bounds");
        }

        buf.copy_from_slice(&self.data[file_offset..file_offset + buf.len()]);
        Ok(())
    }
}

pub enum Input {
    Process(i32),
    Dump(PathBuf),
    ConcatDump(PathBuf),
    MachoCore(PathBuf),
}

#[derive(Default)]
pub struct DumpOptions {
    /// Dump all objects instead of only native (/Script/) objects
    pub all: bool,
    /// Dump FName table
    pub names: bool,
    /// Print struct layouts before dumping
    pub verbose: bool,
    /// Filter out objects whose path contains any of these substrings
    pub filter_out_paths: Vec<String>,
}

pub fn dump(
    input: Input,
    overrides: ConfigOverrides,
    struct_info: Option<Structs>,
    options: DumpOptions,
) -> Result<Jmap> {
    smol::block_on(dump_async(input, overrides, struct_info, options))
}

async fn dump_async(
    input: Input,
    overrides: ConfigOverrides,
    struct_info: Option<Structs>,
    options: DumpOptions,
) -> Result<Jmap> {
    match input {
        Input::Process(pid) => {
            let source_name = proc_name::get_process_name(pid).unwrap_or_default();

            let handle: ProcessHandle = ProcessHandle::new(pid);
            let mem = BlockCache::wrap(handle);
            let ctx = match overrides.clone().into_complete() {
                Some(config) => connect_manual(mem, config, struct_info, options.verbose).await?,
                None => {
                    let image = patternsleuth::process::external::read_image_from_pid(pid)?;
                    connect(mem, &image, overrides, struct_info, options.verbose).await?
                }
            };
            #[cfg(any(target_os = "windows", target_os = "linux"))]
            let _suspend_guard = mem::ProcessSuspendGuard::suspend(pid)?;
            dump_inner(ctx, &source_name, options).await
        }
        Input::Dump(path) => {
            let source_name = path.file_name().unwrap_or_default().to_string_lossy();

            let ctx = match overrides.clone().into_complete() {
                Some(config) => {
                    let mem = MinidumpMem::new(load_minidump(&path)?)?;
                    connect_manual(mem, config, struct_info, options.verbose).await?
                }
                None => {
                    let dump = open_minidump(&path)?;
                    let mem = MinidumpMem::new(dump.minidump)?;
                    connect(mem, &dump.image, overrides, struct_info, options.verbose).await?
                }
            };
            dump_inner(ctx, &source_name, options).await
        }
        Input::ConcatDump(path) => {
            let config = overrides.into_complete().context(
                "concat dumps require manual config (--fname-pool, --guobject-array, --engine-version)",
            )?;
            let source_name = path.file_name().unwrap_or_default().to_string_lossy();
            let mem = ConcatMem::open(&path)?;
            let ctx = connect_manual(mem, config, struct_info, options.verbose).await?;
            dump_inner(ctx, &source_name, options).await
        }
        Input::MachoCore(path) => {
            let mut overrides = overrides;
            overrides
                .target_triplet
                .get_or_insert_with(structs::macos_target_triplet);
            let config = overrides.into_complete().context(
                "Mach-O cores require manual config (--fname-pool, --guobject-array, --engine-version)",
            )?;
            let source_name = path.file_name().unwrap_or_default().to_string_lossy();
            let mem = BlockCache::wrap(MachoCoreMem::open(&path)?);
            let ctx = connect_manual(mem, config, struct_info, options.verbose).await?;
            dump_inner(ctx, &source_name, options).await
        }
    }
}

use script_containers::*;
mod script_containers {
    use super::*;

    #[derive(Clone, Copy)]
    pub struct FScriptArray;
    impl Ptr<FScriptArray> {
        pub fn data(&self) -> Ptr<Option<Ptr<()>>> {
            self.byte_offset(0).cast()
        }
        pub fn num(&self) -> Ptr<u32> {
            self.byte_offset(8).cast()
        }
    }
}

fn print_struct_layouts(structs: &Structs) {
    eprintln!("=== Struct Layouts ===");
    for info in &structs.0 {
        eprintln!(
            "{} size=0x{:x} align=0x{:x}",
            info.name, info.size, info.alignment
        );
        for member in &info.members {
            eprintln!("  0x{:<4x} {}", member.offset, member.name);
        }
    }
}

pub async fn connect_pid(pid: i32, struct_info: Option<Structs>) -> Result<Ctx> {
    let handle: ProcessHandle = ProcessHandle::new(pid);
    let mem = BlockCache::wrap(handle);
    let image = patternsleuth::process::external::read_image_from_pid(pid)?;
    connect(mem, &image, ConfigOverrides::default(), struct_info, false).await
}

pub async fn connect_pid_live(pid: i32, struct_info: Option<Structs>) -> Result<Ctx> {
    let handle: ProcessHandle = ProcessHandle::new(pid);
    let image = patternsleuth::process::external::read_image_from_pid(pid)?;
    connect(
        handle,
        &image,
        ConfigOverrides::default(),
        struct_info,
        false,
    )
    .await
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Config {
    pub guobject_array: u64,
    pub fname_pool: u64,
    pub engine_version: (u16, u16),
    #[serde(default)]
    pub image_base: u64,
    #[serde(default)]
    pub build_change_list: Option<String>,
    #[serde(default)]
    pub case_preserving: bool,
    #[serde(default)]
    pub pack_fuobject_item: bool,
    #[serde(default = "structs::default_target_triplet")]
    pub target_triplet: structs::TargetTriplet,
}

#[derive(Debug, Clone, Default)]
pub struct ConfigOverrides {
    pub guobject_array: Option<u64>,
    pub fname_pool: Option<u64>,
    pub engine_version: Option<(u16, u16)>,
    pub image_base: Option<u64>,
    pub build_change_list: Option<String>,
    pub case_preserving: Option<bool>,
    pub pack_fuobject_item: Option<bool>,
    /// Target triple: `None` defaults to win64 MSVC
    pub target_triplet: Option<structs::TargetTriplet>,
}

impl ConfigOverrides {
    pub fn into_complete(self) -> Option<Config> {
        Some(Config {
            guobject_array: self.guobject_array?,
            fname_pool: self.fname_pool?,
            engine_version: self.engine_version?,
            image_base: self.image_base.unwrap_or(0),
            build_change_list: self.build_change_list,
            case_preserving: self.case_preserving.unwrap_or(false),
            pack_fuobject_item: self.pack_fuobject_item.unwrap_or(false),
            target_triplet: self
                .target_triplet
                .unwrap_or_else(structs::default_target_triplet),
        })
    }
}

async fn read_u32<M: mem::Mem + ?Sized>(mem: &M, addr: u64) -> Result<u32> {
    let mut buf = [0u8; 4];
    mem.read_buf(addr, &mut buf).await?;
    Ok(u32::from_le_bytes(buf))
}

pub async fn resolve_config(
    mem: &impl mem::Mem,
    image: &Image<'_>,
    overrides: &ConfigOverrides,
) -> Result<Config> {
    let results = resolve(image, Resolution::resolver())?;
    println!("{results:X?}");

    let engine_version = overrides.engine_version.or_else(|| {
        results
            .engine_version
            .as_ref()
            .ok()
            .map(|v| (v.major, v.minor))
    });
    let guobject_array = overrides
        .guobject_array
        .or_else(|| results.guobject_array.as_ref().ok().map(|r| r.0));
    let fname_pool = overrides
        .fname_pool
        .or_else(|| results.fname_pool.as_ref().ok().map(|r| r.0));

    let mut missing = Vec::new();
    if engine_version.is_none() {
        missing.push("--engine-version");
    }
    if guobject_array.is_none() {
        missing.push("--guobject-array");
    }
    if fname_pool.is_none() {
        missing.push("--fname-pool");
    }
    if !missing.is_empty() {
        bail!(
            "patternsleuth could not resolve {} and no manual value was supplied; pass {} to fill in the missing value(s)",
            missing
                .iter()
                .map(|f| f.trim_start_matches("--"))
                .collect::<Vec<_>>()
                .join(", "),
            missing.join(" "),
        );
    }
    let engine_version = engine_version.unwrap();
    let guobject_array = guobject_array.unwrap();
    let fname_pool = fname_pool.unwrap();

    let case_preserving = match overrides.case_preserving {
        Some(cp) => cp,
        None => detect_case_preserving(mem, &results, engine_version).await?,
    };

    Ok(Config {
        guobject_array,
        fname_pool,
        engine_version,
        image_base: overrides.image_base.unwrap_or(image.base_address),
        build_change_list: overrides
            .build_change_list
            .clone()
            .or_else(|| results.build.as_ref().ok().map(|cl| cl.0.clone())),
        case_preserving,
        pack_fuobject_item: overrides.pack_fuobject_item.unwrap_or(false),
        target_triplet: overrides
            .target_triplet
            .unwrap_or_else(structs::default_target_triplet),
    })
}

async fn detect_case_preserving(
    mem: &impl mem::Mem,
    results: &Resolution,
    version: (u16, u16),
) -> Result<bool> {
    let Ok(fname_constant) = &results.fname_constant else {
        return Ok(false);
    };
    let name_constant_address = fname_constant.0;

    // Field offsets mirror the FName layout in jmap_dumper/unreal/src/unreal.gs:
    //         UE <  4.23: [CMP, NUM]
    // 4.23 <= UE <  5.01: [CMP, DISP?, NUM]   (DISP only when case-preserving)
    //         UE >= 5.01: [CMP, NUM, DISP?]
    let comparison_index = read_u32(mem, name_constant_address).await?;
    assert_ne!(comparison_index, 0);

    let (case_preserving_detected, number_off) = if version < (4, 23) {
        (false, 4)
    } else if version < (5, 1) {
        if read_u32(mem, name_constant_address + 4).await? == comparison_index {
            (true, 8)
        } else {
            (false, 4)
        }
    } else {
        let cp = read_u32(mem, name_constant_address + 8).await? == comparison_index;
        (cp, 4)
    };

    let number = read_u32(mem, name_constant_address + number_off).await?;
    assert_eq!(
        number, 0,
        "Builds with outlined name number (UE_FNAME_OUTLINE_NUMBER=1) are not supported"
    );
    Ok(case_preserving_detected)
}

pub async fn connect(
    mem: impl mem::Mem + 'static,
    image: &Image<'_>,
    overrides: ConfigOverrides,
    struct_info: Option<Structs>,
    verbose: bool,
) -> Result<Ctx> {
    let config = resolve_config(&mem, image, &overrides).await?;
    connect_manual(mem, config, struct_info, verbose).await
}

pub async fn connect_manual(
    mem: impl mem::Mem + 'static,
    config: Config,
    struct_info: Option<Structs>,
    verbose: bool,
) -> Result<Ctx> {
    let engine_version = patternsleuth::resolvers::unreal::engine_version::EngineVersion {
        major: config.engine_version.0,
        minor: config.engine_version.1,
    };

    let struct_info = if let Some(provided_info) = struct_info {
        provided_info
    } else {
        structs::get_struct_info_for_version(
            &engine_version,
            config.case_preserving,
            config.pack_fuobject_item,
            config.target_triplet,
        )
        .with_context(|| {
            format!("Failed to compute struct offsets via Gospel for {engine_version:?}")
        })?
    };

    if verbose {
        print_struct_layouts(&struct_info);
    }

    Ok(Ctx::new(mem::CtxInner {
        mem: Box::new(mem),
        fnamepool: config.fname_pool,
        structs: struct_info
            .0
            .into_iter()
            .map(|s| (s.name.clone(), s))
            .collect(),
        version: config.engine_version,
        case_preserving: config.case_preserving,
        uobjectarray: config.guobject_array,
        image_base_address: config.image_base,
        build_change_list: config.build_change_list,
    }))
}

/// Insert an object into the map, handling path collisions.
///
/// UE normally guarantees one UObject per path, but plugins can break this by
/// calling `UObjectBase::LowLevelRename` to collide their own UClass with a
/// stock one (e.g. RedpointEOS renames `UOnlineEngineInterfaceEOS` onto
/// `/Script/OnlineSubsystemUtils.OnlineEngineInterfaceImpl`). When that
/// happens, we prefer the UClass whose CDO still lives at the canonical
/// `{class_outer}.Default__{class_name}` path — the renamed class's CDO kept
/// its original outer, so it fails this check.
fn insert_object(objects: &mut BTreeMap<String, ObjectType>, path: String, object: ObjectType) {
    use std::collections::btree_map::Entry;

    match objects.entry(path) {
        Entry::Vacant(e) => {
            e.insert(object);
        }
        Entry::Occupied(mut e) => {
            let path = e.key().clone();
            let existing = e.get();
            let prefer_new =
                has_canonical_cdo(&path, &object) && !has_canonical_cdo(&path, existing);
            eprintln!(
                "WARN: path collision {path}: existing {}, new {}",
                existing.get_object().address,
                object.get_object().address,
            );
            if prefer_new {
                e.insert(object);
            }
        }
    }
}

fn has_canonical_cdo(class_path: &str, obj: &ObjectType) -> bool {
    let ObjectType::Class(c) = obj else {
        return false;
    };
    let (outer, name) = match class_path.rsplit_once(['.', ':']) {
        Some(split) => split,
        None => return false,
    };
    let expected = format!("{outer}.Default__{name}");
    c.class_default_object.as_deref() == Some(expected.as_str())
}

/// Number of object dumps to keep in flight at once
fn dump_concurrency() -> usize {
    std::env::var("JMAP_CONCURRENCY")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .filter(|&n| n > 0)
        .unwrap_or(1024)
}

async fn dump_one(
    uobjectarray: &Ptr<FUObjectArray>,
    i: usize,
    num: usize,
    options: &DumpOptions,
) -> Result<Option<(String, ObjectType)>> {
    let Some(obj) = uobjectarray.read_item_ptr(i).await? else {
        return Ok(None);
    };

    let path = obj.path().await?;

    let filtered = options.filter_out_paths.iter().any(|f| path.contains(f.as_str()));

    if options.verbose {
        if filtered {
            eprintln!("[{i}/{num}] [filtered] {path}");
        } else {
            eprintln!("[{i}/{num}] {path}");
        }
    }

    if filtered {
        return Ok(None);
    }

    Ok(read_object_type(obj, &path, options)
        .await?
        .map(|object| (path, object)))
}

async fn dump_inner(mem: Ctx, source_name: &str, options: DumpOptions) -> Result<Jmap> {
    let uobjectarray = Ptr::<FUObjectArray>::new(mem.uobjectarray, mem.clone())?;

    let mut objects = BTreeMap::<String, ObjectType>::default();
    let mut child_map = HashMap::<String, BTreeSet<String>>::default();

    let num = uobjectarray.num_elements().await? as usize;

    // keep many object dumps in flight
    let mut stream = futures_util::stream::iter(0..num)
        .map(|i| dump_one(&uobjectarray, i, num, &options))
        .buffer_unordered(dump_concurrency());

    while let Some(result) = stream.next().await {
        if let Some((path, object)) = result? {
            if let Some(outer) = object.get_object().outer.clone() {
                child_map.entry(outer).or_default().insert(path.clone());
            }
            insert_object(&mut objects, path, object);
        }
    }
    drop(stream);

    for (outer, children) in child_map {
        if let Some(outer) = objects.get_mut(&outer) {
            match outer {
                ObjectType::Package(obj) => &mut obj.object,
                ObjectType::Enum(obj) => &mut obj.object,
                ObjectType::ScriptStruct(obj) => &mut obj.r#struct.object,
                ObjectType::Class(obj) => &mut obj.r#struct.object,
                ObjectType::Function(obj) => &mut obj.r#struct.object,
                ObjectType::Object(obj) => obj,
            }
            .children = children;
        }
    }

    let vtables = vtable::analyze_vtables(&mem, &mut objects).await;

    let names = if options.names {
        Some(extract_fnames(&mem).await?)
    } else {
        None
    };

    Ok(Jmap {
        metadata: Some(Metadata {
            tool: "https://github.com/trumank/jmap".to_string(),
            timestamp: time::OffsetDateTime::now_utc().to_string(),
            source: source_name.to_string(),
            engine_version: EngineVersion {
                major: mem.version.0,
                minor: mem.version.1,
            },
            build_change_list: mem.build_change_list.clone(),
        }),
        image_base_address: mem.image_base_address.into(),
        objects,
        vtables,
        names,
    })
}

pub async fn read_object_type(
    obj: Ptr<UObject>,
    path: &str,
    options: &DumpOptions,
) -> Result<Option<ObjectType>> {
    if !options.all && !path.starts_with("/Script/") {
        return Ok(None);
    }
    if options.filter_out_paths.iter().any(|f| path.contains(f.as_str())) {
        return Ok(None);
    }

    let class = obj.class_private().read().await?;
    let object_flags = obj.object_flags().read().await?;
    let is_basic_object = object_flags.contains(EObjectFlags::RF_ArchetypeObject)
        || object_flags.contains(EObjectFlags::RF_ClassDefaultObject);

    let f = class.class_cast_flags().read().await?;
    let object = if !is_basic_object && f.contains(EClassCastFlags::CASTCLASS_UClass) {
        ObjectType::Class(read_class(&obj.cast()).await?)
    } else if !is_basic_object && f.contains(EClassCastFlags::CASTCLASS_UFunction) {
        let full_obj = obj.cast::<UFunction>();
        let function_flags = full_obj.function_flags().read().await?;
        ObjectType::Function(Function {
            r#struct: read_struct(&obj.cast()).await?,
            function_flags,
            func: (full_obj.func().read().await? as u64).into(),
        })
    } else if !is_basic_object && f.contains(EClassCastFlags::CASTCLASS_UScriptStruct) {
        ObjectType::ScriptStruct(read_script_struct(&obj.cast()).await?)
    } else if !is_basic_object && f.contains(EClassCastFlags::CASTCLASS_UEnum) {
        ObjectType::Enum(read_enum(&obj.cast()).await?)
    } else if !is_basic_object && f.contains(EClassCastFlags::CASTCLASS_UPackage) {
        ObjectType::Package(Package {
            object: read_object(&obj).await?,
        })
    } else {
        let obj = obj.cast::<UObject>();
        ObjectType::Object(read_object(&obj).await?)
    };
    Ok(Some(object))
}

async fn opt_path<T>(opt: Option<Ptr<T>>) -> Result<Option<String>>
where
    Ptr<T>: HasPath,
{
    Ok(match opt {
        Some(p) => Some(p.path().await?),
        None => None,
    })
}

#[allow(async_fn_in_trait)]
pub trait HasPath {
    async fn path(&self) -> Result<String>;
}
macro_rules! has_path {
    ($($t:ty),* $(,)?) => { $( impl HasPath for Ptr<$t> {
        async fn path(&self) -> Result<String> { Ptr::<$t>::path(self).await }
    } )* };
}
has_path!(UObject, UClass, UStruct, UScriptStruct, UFunction, UEnum);

pub async fn read_prop_type(ptr: &Ptr<ZProperty>) -> Result<Property> {
    let name = ptr.zfield().name_private().read().await?;
    let f = ptr.zfield().cast_flags().await?;

    let t = if f.contains(EClassCastFlags::CASTCLASS_FStructProperty) {
        let prop = ptr.cast::<ZStructProperty>();
        let s = prop.struct_().read().await?.path().await?;
        PropertyType::Struct { r#struct: s }
    } else if f.contains(EClassCastFlags::CASTCLASS_FStrProperty) {
        PropertyType::Str
    } else if f.contains(EClassCastFlags::CASTCLASS_FNameProperty) {
        PropertyType::Name
    } else if f.contains(EClassCastFlags::CASTCLASS_FTextProperty) {
        PropertyType::Text
    } else if f.contains(EClassCastFlags::CASTCLASS_FMulticastInlineDelegateProperty) {
        let prop = ptr.cast::<ZMulticastDelegateProperty>();
        let signature_function = opt_path(prop.signature_function().read().await?).await?;
        PropertyType::MulticastInlineDelegate { signature_function }
    } else if f.contains(EClassCastFlags::CASTCLASS_FMulticastSparseDelegateProperty) {
        let prop = ptr.cast::<ZMulticastDelegateProperty>();
        let signature_function = opt_path(prop.signature_function().read().await?).await?;
        PropertyType::MulticastSparseDelegate { signature_function }
    } else if f.contains(EClassCastFlags::CASTCLASS_FMulticastDelegateProperty) {
        let prop = ptr.cast::<ZMulticastDelegateProperty>();
        let signature_function = opt_path(prop.signature_function().read().await?).await?;
        PropertyType::MulticastDelegate { signature_function }
    } else if f.contains(EClassCastFlags::CASTCLASS_FDelegateProperty) {
        let prop = ptr.cast::<ZDelegateProperty>();
        let signature_function = opt_path(prop.signature_function().read().await?).await?;
        PropertyType::Delegate { signature_function }
    } else if f.contains(EClassCastFlags::CASTCLASS_FBoolProperty) {
        let prop = ptr.cast::<ZBoolProperty>();
        PropertyType::Bool {
            field_size: prop.field_size().read().await?,
            byte_offset: prop.byte_offset_().read().await?,
            byte_mask: prop.byte_mask().read().await?,
            field_mask: prop.field_mask().read().await?,
        }
    } else if f.contains(EClassCastFlags::CASTCLASS_FArrayProperty) {
        let prop = ptr.cast::<ZArrayProperty>();
        PropertyType::Array {
            inner: Box::pin(read_prop_type(&prop.inner().read().await?.cast()))
                .await?
                .into(),
        }
    } else if f.contains(EClassCastFlags::CASTCLASS_FEnumProperty) {
        let prop = ptr.cast::<ZEnumProperty>();
        PropertyType::Enum {
            container: Box::pin(read_prop_type(&prop.underlying_prop().read().await?.cast()))
                .await?
                .into(),
            r#enum: opt_path(prop.enum_().read().await?).await?,
        }
    } else if f.contains(EClassCastFlags::CASTCLASS_FMapProperty) {
        let prop = ptr.cast::<ZMapProperty>();
        PropertyType::Map {
            key_prop: Box::pin(read_prop_type(&prop.key_prop().read().await?.cast()))
                .await?
                .into(),
            value_prop: Box::pin(read_prop_type(&prop.value_prop().read().await?.cast()))
                .await?
                .into(),
        }
    } else if f.contains(EClassCastFlags::CASTCLASS_FSetProperty) {
        let prop = ptr.cast::<ZSetProperty>();
        PropertyType::Set {
            key_prop: Box::pin(read_prop_type(&prop.element_prop().read().await?.cast()))
                .await?
                .into(),
        }
    } else if f.contains(EClassCastFlags::CASTCLASS_FFloatProperty) {
        PropertyType::Float
    } else if f.contains(EClassCastFlags::CASTCLASS_FDoubleProperty) {
        PropertyType::Double
    } else if f.contains(EClassCastFlags::CASTCLASS_FByteProperty) {
        let prop = ptr.cast::<ZByteProperty>();
        PropertyType::Byte {
            r#enum: opt_path(prop.enum_().read().await?).await?,
        }
    } else if f.contains(EClassCastFlags::CASTCLASS_FUInt16Property) {
        PropertyType::UInt16
    } else if f.contains(EClassCastFlags::CASTCLASS_FUInt32Property) {
        PropertyType::UInt32
    } else if f.contains(EClassCastFlags::CASTCLASS_FUInt64Property) {
        PropertyType::UInt64
    } else if f.contains(EClassCastFlags::CASTCLASS_FInt8Property) {
        PropertyType::Int8
    } else if f.contains(EClassCastFlags::CASTCLASS_FInt16Property) {
        PropertyType::Int16
    } else if f.contains(EClassCastFlags::CASTCLASS_FIntProperty) {
        PropertyType::Int
    } else if f.contains(EClassCastFlags::CASTCLASS_FInt64Property) {
        PropertyType::Int64
    } else if f.contains(EClassCastFlags::CASTCLASS_FClassProperty) {
        let prop = ptr.cast::<ZClassProperty>();
        let property_class = prop
            .fobject_property()
            .property_class()
            .read()
            .await?
            .path()
            .await?;
        let meta_class = prop.meta_class().read().await?.path().await?;
        PropertyType::Class {
            property_class,
            meta_class,
        }
    } else if f.contains(EClassCastFlags::CASTCLASS_FObjectProperty) {
        let prop = ptr.cast::<ZObjectProperty>();
        let property_class = prop.property_class().read().await?.path().await?;
        PropertyType::Object { property_class }
    } else if f.contains(EClassCastFlags::CASTCLASS_FSoftClassProperty) {
        let prop = ptr.cast::<ZSoftClassProperty>();
        let property_class = prop
            .fsoft_object_property()
            .property_class()
            .read()
            .await?
            .path()
            .await?;
        let meta_class = prop.meta_class().read().await?.path().await?;
        PropertyType::SoftClass {
            property_class,
            meta_class,
        }
    } else if f.contains(EClassCastFlags::CASTCLASS_FSoftObjectProperty) {
        let prop = ptr.cast::<ZSoftObjectProperty>();
        let property_class = prop.property_class().read().await?.path().await?;
        PropertyType::SoftObject { property_class }
    } else if f.contains(EClassCastFlags::CASTCLASS_FWeakObjectProperty) {
        let prop = ptr.cast::<ZWeakObjectProperty>();
        let c = prop.property_class().read().await?.path().await?;
        PropertyType::WeakObject { property_class: c }
    } else if f.contains(EClassCastFlags::CASTCLASS_FLazyObjectProperty) {
        let prop = ptr.cast::<ZLazyObjectProperty>();
        let c = prop.property_class().read().await?.path().await?;
        PropertyType::LazyObject { property_class: c }
    } else if f.contains(EClassCastFlags::CASTCLASS_FInterfaceProperty) {
        let prop = ptr.cast::<ZInterfaceProperty>();
        let interface_class = prop.interface_class().read().await?.path().await?;
        PropertyType::Interface { interface_class }
    } else if f.contains(EClassCastFlags::CASTCLASS_FFieldPathProperty) {
        // TODO
        PropertyType::FieldPath
    } else if f.contains(EClassCastFlags::CASTCLASS_FOptionalProperty) {
        let prop = ptr.cast::<FOptionalProperty>();
        PropertyType::Optional {
            inner: Box::pin(read_prop_type(&prop.value_property().read().await?.cast()))
                .await?
                .into(),
        }
    } else if f.contains(EClassCastFlags::CASTCLASS_FUtf8StrProperty) {
        PropertyType::Utf8Str
    } else if f.contains(EClassCastFlags::CASTCLASS_FAnsiStrProperty) {
        PropertyType::AnsiStr
    } else {
        unimplemented!("{f:?}");
    };

    let prop = ptr.cast::<ZProperty>();
    Ok(Property {
        address: ptr.address().into(),
        name,
        offset: prop.offset_internal().read().await? as usize,
        array_dim: prop.array_dim().read().await? as usize,
        size: prop.element_size().read().await? as usize,
        flags: prop.property_flags().read().await?,
        r#type: t,
    })
}

pub async fn read_props(
    ustruct: &Ptr<UStruct>,
    ptr: &Ptr<()>,
) -> Result<OrderMap<String, PropertyValue>> {
    let mut properties = OrderMap::new();
    let mut props = ustruct.properties(true);
    while let Some(prop) = props.next().await {
        let prop = prop?;
        let array_dim = prop.array_dim().read().await? as usize;
        let name = prop.zfield().name_private().read().await?;
        if array_dim == 1 {
            if let Some(value) = Box::pin(read_prop(&prop, ptr, 0)).await? {
                properties.insert(name, value);
            }
        } else {
            let mut elements = vec![];
            let mut success = true;
            for i in 0..array_dim {
                if let Some(value) = Box::pin(read_prop(&prop, ptr, i)).await? {
                    elements.push(value);
                } else {
                    success = false;
                }
            }
            if success {
                properties.insert(name, PropertyValue::Array(elements));
            }
        }
    }
    Ok(properties)
}

pub async fn read_prop(
    prop: &Ptr<ZProperty>,
    ptr: &Ptr<()>,
    index: usize,
) -> Result<Option<PropertyValue>> {
    let size = prop.element_size().read().await? as usize;
    let ptr = ptr.byte_offset(prop.offset_internal().read().await? as usize + index * size);
    let f = prop.zfield().cast_flags().await?;

    let value = if f.contains(EClassCastFlags::CASTCLASS_FStructProperty) {
        let prop = prop.cast::<ZStructProperty>();
        PropertyValue::Struct(
            Box::pin(read_props(&prop.struct_().read().await?.ustruct(), &ptr)).await?,
        )
    } else if f.contains(EClassCastFlags::CASTCLASS_FStrProperty) {
        PropertyValue::Str(ptr.cast::<FString>().read().await?)
    } else if f.contains(EClassCastFlags::CASTCLASS_FNameProperty) {
        PropertyValue::Name(ptr.cast::<FName>().read().await?)
    } else if f.contains(EClassCastFlags::CASTCLASS_FTextProperty) {
        return Ok(None);
    } else if f.contains(EClassCastFlags::CASTCLASS_FMulticastInlineDelegateProperty) {
        return Ok(None);
    } else if f.contains(EClassCastFlags::CASTCLASS_FMulticastSparseDelegateProperty) {
        return Ok(None);
    } else if f.contains(EClassCastFlags::CASTCLASS_FMulticastDelegateProperty) {
        return Ok(None);
    } else if f.contains(EClassCastFlags::CASTCLASS_FDelegateProperty) {
        return Ok(None);
    } else if f.contains(EClassCastFlags::CASTCLASS_FBoolProperty) {
        let prop = prop.cast::<ZBoolProperty>();
        let byte_offset = prop.byte_offset_().read().await?;
        let byte_mask = prop.byte_mask().read().await?;
        let byte = ptr
            .byte_offset(byte_offset as usize)
            .cast::<u8>()
            .read()
            .await?;
        PropertyValue::Bool(byte & byte_mask != 0)
    } else if f.contains(EClassCastFlags::CASTCLASS_FArrayProperty) {
        let prop = prop.cast::<ZArrayProperty>();
        let array = ptr.cast::<FScriptArray>();

        let num = array.num().read().await? as usize;
        let mut data = Vec::with_capacity(num);
        if let Some(data_ptr) = array.data().read().await? {
            let inner_prop = prop.inner().read().await?;
            for i in 0..num {
                let value = Box::pin(read_prop(&inner_prop, &data_ptr, i)).await?;
                if let Some(value) = value {
                    data.push(value);
                } else {
                    return Ok(None);
                }
            }
        }

        PropertyValue::Array(data)
    } else if f.contains(EClassCastFlags::CASTCLASS_FEnumProperty) {
        let prop = prop.cast::<ZEnumProperty>();
        let underlying = Box::pin(read_prop(&prop.underlying_prop().read().await?, &ptr, 0))
            .await?
            .expect("valid underlying prop");
        let value = match underlying {
            PropertyValue::Byte(BytePropertyValue::Value(v)) => v as i64,
            PropertyValue::Int8(v) => v as i64,
            PropertyValue::Int16(v) => v as i64,
            PropertyValue::Int(v) => v as i64,
            PropertyValue::Int64(v) => v,
            PropertyValue::UInt16(v) => v as i64,
            PropertyValue::UInt32(v) => v as i64,
            PropertyValue::UInt64(v) => v as i64,
            e => bail!("underlying enum prop {e:?}"),
        };
        let names = read_enum(&prop.enum_().read().await?.expect("valid enum"))
            .await?
            .names;
        let name = names
            .into_iter()
            .find_map(|(name, v)| (v == value).then_some(name));

        PropertyValue::Enum(if let Some(name) = name {
            EnumPropertyValue::Name(name)
        } else {
            EnumPropertyValue::Value(value)
        })
    } else if f.contains(EClassCastFlags::CASTCLASS_FMapProperty) {
        let prop = prop.cast::<ZMapProperty>();
        let map = ptr.cast::<FScriptMap>();

        let key_prop = prop.key_prop().read().await?;
        let value_prop = prop.value_prop().read().await?;

        let map_layout = prop.map_layout();
        let pair_wrapper_size = map_layout.set_layout().size().read().await? as usize;

        let mut entries = BTreeMap::new();

        let sparse_array = map.pairs().elements();
        let max_index = sparse_array.get_max_index().await?;

        for i in 0..max_index {
            if sparse_array.is_valid_index(i).await? {
                let pair_ptr = sparse_array.get_data(i, pair_wrapper_size).await?;

                let key = Box::pin(read_prop(&key_prop, &pair_ptr.cast(), 0)).await?;
                let value = Box::pin(read_prop(&value_prop, &pair_ptr.cast(), 0)).await?;

                if let (Some(k), Some(v)) = (key, value) {
                    entries.insert(k, v);
                }
            }
        }

        PropertyValue::Map(entries)
    } else if f.contains(EClassCastFlags::CASTCLASS_FSetProperty) {
        let prop = prop.cast::<ZSetProperty>();
        let set = ptr.cast::<FScriptSet>();

        let element_prop = prop.element_prop().read().await?;

        let element_wrapper_size = prop.set_layout().size().read().await? as usize;

        let mut elements = BTreeSet::new();

        let sparse_array = set.elements();
        let max_index = sparse_array.get_max_index().await?;

        for i in 0..max_index {
            if sparse_array.is_valid_index(i).await? {
                let element_ptr = sparse_array.get_data(i, element_wrapper_size).await?;
                if let Some(value) =
                    Box::pin(read_prop(&element_prop, &element_ptr.cast(), 0)).await?
                {
                    elements.insert(value);
                }
            }
        }

        PropertyValue::Set(elements)
    } else if f.contains(EClassCastFlags::CASTCLASS_FFloatProperty) {
        PropertyValue::Float(ptr.cast::<f32>().read().await?.into())
    } else if f.contains(EClassCastFlags::CASTCLASS_FDoubleProperty) {
        PropertyValue::Double(ptr.cast::<f64>().read().await?.into())
    } else if f.contains(EClassCastFlags::CASTCLASS_FByteProperty) {
        let prop = prop.cast::<ZByteProperty>();
        let value = ptr.cast::<u8>().read().await?;
        let enum_names = match prop.enum_().read().await? {
            Some(e) => Some(read_enum(&e).await?),
            None => None,
        };
        PropertyValue::Byte(
            if let Some(name) = enum_names.and_then(|e| {
                e.names
                    .into_iter()
                    .find_map(|(name, v)| (v == value as i64).then_some(name))
            }) {
                BytePropertyValue::Name(name)
            } else {
                BytePropertyValue::Value(value)
            },
        )
    } else if f.contains(EClassCastFlags::CASTCLASS_FUInt16Property) {
        PropertyValue::UInt16(ptr.cast::<u16>().read().await?)
    } else if f.contains(EClassCastFlags::CASTCLASS_FUInt32Property) {
        PropertyValue::UInt32(ptr.cast::<u32>().read().await?)
    } else if f.contains(EClassCastFlags::CASTCLASS_FUInt64Property) {
        PropertyValue::UInt64(ptr.cast::<u64>().read().await?)
    } else if f.contains(EClassCastFlags::CASTCLASS_FInt8Property) {
        PropertyValue::Int8(ptr.cast::<i8>().read().await?)
    } else if f.contains(EClassCastFlags::CASTCLASS_FInt16Property) {
        PropertyValue::Int16(ptr.cast::<i16>().read().await?)
    } else if f.contains(EClassCastFlags::CASTCLASS_FIntProperty) {
        PropertyValue::Int(ptr.cast::<i32>().read().await?)
    } else if f.contains(EClassCastFlags::CASTCLASS_FInt64Property) {
        PropertyValue::Int64(ptr.cast::<i64>().read().await?)
    } else if f.contains(EClassCastFlags::CASTCLASS_FObjectProperty) {
        let obj = opt_path(ptr.cast::<Option<Ptr<UObject>>>().read().await?).await?;
        PropertyValue::Object(obj)
    } else if f.contains(EClassCastFlags::CASTCLASS_FWeakObjectProperty) {
        return Ok(None);
    } else if f.contains(EClassCastFlags::CASTCLASS_FSoftObjectProperty) {
        return Ok(None);
    } else if f.contains(EClassCastFlags::CASTCLASS_FLazyObjectProperty) {
        return Ok(None);
    } else if f.contains(EClassCastFlags::CASTCLASS_FInterfaceProperty) {
        return Ok(None);
    } else if f.contains(EClassCastFlags::CASTCLASS_FFieldPathProperty) {
        return Ok(None);
    } else if f.contains(EClassCastFlags::CASTCLASS_FOptionalProperty) {
        return Ok(None);
    } else if f.contains(EClassCastFlags::CASTCLASS_FUtf8StrProperty) {
        PropertyValue::Utf8Str(ptr.cast::<FUtf8String>().read().await?)
    } else if f.contains(EClassCastFlags::CASTCLASS_FAnsiStrProperty) {
        // technically needs to be C locale but probably never going to encounter non-ASCII characters anyway
        PropertyValue::Utf8Str(ptr.cast::<FUtf8String>().read().await?)
    } else {
        unimplemented!("{f:?}");
    };
    Ok(Some(value))
}

pub async fn read_object(obj: &Ptr<UObject>) -> Result<Object> {
    let outer = opt_path(obj.outer_private().read().await?).await?;

    let class = obj.class_private().read().await?;
    let class_name = class.path().await?;

    Ok(Object {
        address: obj.address().into(),
        vtable: (obj.vtable().read().await? as u64).into(),
        object_flags: obj.object_flags().read().await?,
        outer,
        class: class_name,
        children: Default::default(),
        property_values: read_props(&class.ustruct(), &obj.cast()).await?.into(),
    })
}

pub async fn read_struct(obj: &Ptr<UStruct>) -> Result<Struct> {
    let mut properties = vec![];
    let mut props = obj.properties(false);
    while let Some(prop) = props.next().await {
        let prop = prop?;
        let f = prop.zfield().cast_flags().await?;
        if f.contains(EClassCastFlags::CASTCLASS_FProperty) {
            properties.push(read_prop_type(&prop.cast::<ZProperty>()).await?);
        }
    }

    let super_struct = opt_path(obj.super_struct().read().await?).await?;
    Ok(Struct {
        object: read_object(&obj.cast()).await?,
        super_struct,
        properties,
        properties_size: obj.properties_size().read().await? as usize,
        min_alignment: obj.min_alignment().read().await? as usize,
        script: obj.script().read_vec().await?,
    })
}

pub async fn read_script_struct(obj: &Ptr<UScriptStruct>) -> Result<ScriptStruct> {
    Ok(ScriptStruct {
        r#struct: read_struct(&obj.ustruct()).await?,
        struct_flags: obj.struct_flags().read().await?,
    })
}

pub async fn read_class(obj: &Ptr<UClass>) -> Result<Class> {
    let class_flags = obj.class_flags().read().await?;
    let class_cast_flags = obj.class_cast_flags().read().await?;
    let class_default_object = opt_path(obj.class_default_object().read().await?).await?;
    Ok(Class {
        r#struct: read_struct(&obj.cast()).await?,
        class_flags,
        class_cast_flags,
        class_default_object,
        instance_vtable: None,
    })
}

pub async fn read_enum(obj: &Ptr<UEnum>) -> Result<Enum> {
    let enum_flags = if obj.ctx().ue_version() >= (4, 26) {
        Some(obj.enum_flags().read().await?)
    } else {
        None
    };
    Ok(Enum {
        object: read_object(&obj.cast()).await?,
        cpp_type: obj.cpp_type().read().await?,
        cpp_form: obj.cpp_form().read().await?,
        enum_flags,
        names: obj.read_names().await?,
    })
}
