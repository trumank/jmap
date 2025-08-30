#![feature(arbitrary_self_types)]

include!(concat!(env!("OUT_DIR"), "/", "gospel_bindings.rs"));

mod containers;
mod gospel;
mod header;
mod mem;
mod vtable;

use gospel_runtime::static_type_wrappers::Ref;
pub use header::into_header;

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use anyhow::{Context, Result, bail};
use gospel_runtime::memory_access::{DataEndianness, OpaquePtr};
use gospel_runtime::runtime_type_model::TypePtrNamespace;
use gospel_runtime::vm_integration::GospelVMTypeGraphBackend;
use gospel_typelib::type_model::{TargetTriplet, TypeLayoutCache};
use gospel_vm::vm::GospelVMOptions;
use mem::{CtxPtr, Mem, MemCache, Ptr};
use ordermap::OrderMap;
use patternsleuth::image::Image;
use patternsleuth::resolvers::{impl_try_collector, resolve};
use read_process_memory::{Pid, ProcessHandle};
use ue_reflection::{
    BytePropertyValue, Class, EClassCastFlags, EClassFlags, EFunctionFlags, EObjectFlags,
    EPropertyFlags, EStructFlags, Enum, EnumPropertyValue, Function, Object, ObjectType, Package,
    Property, PropertyType, PropertyValue, ReflectionData, ScriptStruct, Struct,
};

use crate::containers::PtrFNamePool;
use crate::gospel_bindings::*;
use crate::mem::Ctx;

impl_try_collector! {
    #[derive(Debug, PartialEq, Clone)]
    struct Resolution {
        guobject_array: patternsleuth::resolvers::unreal::guobject_array::GUObjectArray,
        fname_pool: patternsleuth::resolvers::unreal::fname::FNamePool,
        engine_version: patternsleuth::resolvers::unreal::engine_version::EngineVersion,
    }
}

fn map_prop<C: Ctx>(ptr: &Ref<C, ZProperty>) -> Result<Property> {
    let field = ptr.cast_checked::<ZField>();
    let name = field.name()?;
    let f = field.cast_flags()?;

    let t = if f.contains(EClassCastFlags::CASTCLASS_FStructProperty) {
        let prop = ptr.cast_checked::<ZStructProperty>();
        let s = prop
            .r_struct()
            .read()?
            .cast_checked::<UObject>()
            .to_ref_checked()
            .path()?;
        PropertyType::Struct { r#struct: s }
    } else if f.contains(EClassCastFlags::CASTCLASS_FStrProperty) {
        PropertyType::Str
    } else if f.contains(EClassCastFlags::CASTCLASS_FNameProperty) {
        PropertyType::Name
    } else if f.contains(EClassCastFlags::CASTCLASS_FTextProperty) {
        PropertyType::Text
    } else if f.contains(EClassCastFlags::CASTCLASS_FMulticastInlineDelegateProperty) {
        let prop = ptr.cast_checked::<ZMulticastDelegateProperty>();
        let signature_function = prop
            .signature_function()
            .read()?
            .to_ref()
            .map(|e| e.cast_checked::<UObject>().path())
            .transpose()?;
        PropertyType::MulticastInlineDelegate { signature_function }
    } else if f.contains(EClassCastFlags::CASTCLASS_FMulticastSparseDelegateProperty) {
        let prop = ptr.cast_checked::<ZMulticastDelegateProperty>();
        let signature_function = prop
            .signature_function()
            .read()?
            .to_ref()
            .map(|e| e.cast_checked::<UObject>().path())
            .transpose()?;
        PropertyType::MulticastSparseDelegate { signature_function }
    } else if f.contains(EClassCastFlags::CASTCLASS_FMulticastDelegateProperty) {
        let prop = ptr.cast_checked::<ZMulticastDelegateProperty>();
        let signature_function = prop
            .signature_function()
            .read()?
            .to_ref()
            .map(|e| e.cast_checked::<UObject>().path())
            .transpose()?;
        PropertyType::MulticastDelegate { signature_function }
    } else if f.contains(EClassCastFlags::CASTCLASS_FDelegateProperty) {
        let prop = ptr.cast_checked::<ZDelegateProperty>();
        let signature_function = prop
            .signature_function()
            .read()?
            .to_ref()
            .map(|e| e.cast_checked::<UObject>().path())
            .transpose()?;
        PropertyType::Delegate { signature_function }
    } else if f.contains(EClassCastFlags::CASTCLASS_FBoolProperty) {
        let prop = ptr.cast_checked::<ZBoolProperty>();
        PropertyType::Bool {
            field_size: prop.field_size().read()?,
            byte_offset: prop.byte_offset().read()?,
            byte_mask: prop.byte_mask().read()?,
            field_mask: prop.field_mask().read()?,
        }
    } else if f.contains(EClassCastFlags::CASTCLASS_FArrayProperty) {
        let prop = ptr.cast_checked::<ZArrayProperty>();
        PropertyType::Array {
            inner: map_prop(&prop.inner().unwrap().read()?.to_ref_checked())?.into(),
        }
    } else if f.contains(EClassCastFlags::CASTCLASS_FEnumProperty) {
        let prop = ptr.cast_checked::<ZEnumProperty>();
        PropertyType::Enum {
            container: map_prop(
                &prop
                    .underlying_prop()
                    .read()?
                    .to_ref_checked()
                    .cast_checked::<ZProperty>(),
            )?
            .into(),
            r#enum: prop
                .r_enum()
                .read()?
                .to_ref()
                .map(|e| e.cast_checked::<UObject>().path())
                .transpose()?,
        }
    } else if f.contains(EClassCastFlags::CASTCLASS_FMapProperty) {
        let prop = ptr.cast_checked::<ZMapProperty>();
        PropertyType::Map {
            key_prop: map_prop(&prop.key_prop().read()?.to_ref_checked())?.into(),
            value_prop: map_prop(&prop.value_prop().read()?.to_ref_checked())?.into(),
        }
    } else if f.contains(EClassCastFlags::CASTCLASS_FSetProperty) {
        let prop = ptr.cast_checked::<ZSetProperty>();
        PropertyType::Set {
            key_prop: map_prop(&prop.element_prop().read()?.to_ref_checked())?.into(),
        }
    } else if f.contains(EClassCastFlags::CASTCLASS_FFloatProperty) {
        PropertyType::Float
    } else if f.contains(EClassCastFlags::CASTCLASS_FDoubleProperty) {
        PropertyType::Double
    } else if f.contains(EClassCastFlags::CASTCLASS_FByteProperty) {
        let prop = ptr.cast_checked::<ZByteProperty>();
        PropertyType::Byte {
            r#enum: prop
                .r_enum()
                .read()?
                .to_ref()
                .map(|e| e.cast_checked::<UObject>().path())
                .transpose()?,
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
        let prop = ptr.cast_checked::<ZClassProperty>();
        let property_class = prop
            .cast_checked::<ZObjectPropertyBase>()
            .property_class()
            .read()?
            .to_ref_checked()
            .cast_checked::<UObject>()
            .path()?;
        let meta_class = prop
            .meta_class()
            .read()?
            .to_ref_checked()
            .cast_checked::<UObject>()
            .path()?;
        PropertyType::Class {
            property_class,
            meta_class,
        }
    } else if f.contains(EClassCastFlags::CASTCLASS_FObjectProperty) {
        let prop = ptr.cast_checked::<ZObjectPropertyBase>();
        let property_class = prop
            .property_class()
            .read()?
            .to_ref_checked()
            .cast_checked::<UObject>()
            .path()?;
        PropertyType::Object { property_class }
    } else if f.contains(EClassCastFlags::CASTCLASS_FSoftClassProperty) {
        let prop = ptr.cast_checked::<ZSoftClassProperty>();
        let property_class = prop
            .cast_checked::<ZObjectPropertyBase>()
            .property_class()
            .read()?
            .to_ref_checked()
            .cast_checked::<UObject>()
            .path()?;
        let meta_class = prop
            .meta_class()
            .read()?
            .to_ref_checked()
            .cast_checked::<UObject>()
            .path()?;
        PropertyType::SoftClass {
            property_class,
            meta_class,
        }
    } else if f.contains(EClassCastFlags::CASTCLASS_FSoftObjectProperty) {
        let prop = ptr.cast_checked::<ZObjectPropertyBase>();
        let property_class = prop
            .property_class()
            .read()?
            .to_ref_checked()
            .cast_checked::<UObject>()
            .path()?;
        PropertyType::SoftObject { property_class }
    } else if f.contains(EClassCastFlags::CASTCLASS_FWeakObjectProperty) {
        let prop = ptr.cast_checked::<ZObjectPropertyBase>();
        let property_class = prop
            .property_class()
            .read()?
            .to_ref_checked()
            .cast_checked::<UObject>()
            .path()?;
        PropertyType::WeakObject { property_class }
    } else if f.contains(EClassCastFlags::CASTCLASS_FLazyObjectProperty) {
        let prop = ptr.cast_checked::<ZObjectPropertyBase>();
        let property_class = prop
            .property_class()
            .read()?
            .to_ref_checked()
            .cast_checked::<UObject>()
            .path()?;
        PropertyType::LazyObject { property_class }
    } else if f.contains(EClassCastFlags::CASTCLASS_FInterfaceProperty) {
        let prop = ptr.cast_checked::<ZInterfaceProperty>();
        let interface_class = prop
            .interface_class()
            .read()?
            .to_ref_checked()
            .cast_checked::<UObject>()
            .path()?;
        PropertyType::Interface { interface_class }
    } else if f.contains(EClassCastFlags::CASTCLASS_FFieldPathProperty) {
        // TODO
        PropertyType::FieldPath
    } else if f.contains(EClassCastFlags::CASTCLASS_FOptionalProperty) {
        let prop = ptr.cast_checked::<FOptionalPropertyLayout>();
        PropertyType::Optional {
            inner: map_prop(&prop.value_property().read()?.to_ref_checked())?.into(),
        }
    } else {
        unimplemented!("{f:?}");
    };

    let prop = ptr;
    Ok(Property {
        name,
        offset: prop.offset_internal().read()? as usize,
        array_dim: prop.array_dim().read()? as usize,
        size: prop.element_size().read()? as usize,
        flags: EPropertyFlags::from_bits(prop.property_flags().unwrap().read()?).unwrap(),
        r#type: t,
    })
}

#[derive(Clone)]
struct MemoryRegion<'a> {
    base_address: u64,
    end_address: u64,
    data: &'a [u8],
}

#[derive(Clone)]
struct MinidumpMem<'a> {
    regions: Arc<Vec<MemoryRegion<'a>>>,
}

impl<'a> MinidumpMem<'a> {
    fn new(minidump: &'a minidump::Minidump<'_, &'a [u8]>) -> Result<Self> {
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

impl Mem for MinidumpMem<'_> {
    fn read_buf(&self, address: u64, buf: &mut [u8]) -> Result<()> {
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

pub enum Input {
    Process(i32),
    Dump(PathBuf),
}

pub fn dump(input: Input) -> Result<ReflectionData> {
    match input {
        Input::Process(pid) => {
            let handle: ProcessHandle = (pid as Pid).try_into()?;
            let mem = MemCache::wrap(handle);
            let image = patternsleuth::process::external::read_image_from_pid(pid)?;
            dump_inner(mem, &image)
        }
        Input::Dump(path) => {
            let file = std::fs::File::open(path)?;
            let mmap = unsafe { memmap2::MmapOptions::new().map(&file)? };

            let minidump = minidump::Minidump::read(&*mmap)?;
            let mem = MinidumpMem::new(&minidump)?;
            let img = patternsleuth::image::pe::read_image_from_minidump(&minidump)?;
            dump_inner(mem, &img)
        }
    }
}

use script_containers::*;
mod script_containers {
    use super::*;

    #[derive(Clone, Copy)]
    pub struct FScriptArray;
    impl<C: Clone + Ctx> Ptr<FScriptArray, C> {
        pub fn data(&self) -> Ptr<Option<Ptr<(), C>>, C> {
            self.byte_offset(0).cast()
        }
        pub fn num(&self) -> Ptr<u32, C> {
            self.byte_offset(8).cast()
        }
    }
}

fn dump_inner<M: Mem>(mem: M, image: &Image<'_>) -> Result<ReflectionData> {
    let results = resolve(image, Resolution::resolver())?;
    println!("{results:X?}");

    let fnamepool = PtrFNamePool(results.fname_pool.0);

    let case_preserving = false;

    let target_triplet = TargetTriplet {
        arch: gospel_typelib::type_model::TargetArchitecture::X86_64,
        sys: gospel_typelib::type_model::TargetOperatingSystem::Win32,
        env: gospel_typelib::type_model::TargetEnvironment::MSVC,
    };

    let ue_version =
        (results.engine_version.major as u64) * 100 + (results.engine_version.minor as u64);
    let case_preserving_flag = if case_preserving { 1 } else { 0 };
    let vm_options = GospelVMOptions::default()
        .target_triplet(target_triplet.clone())
        .with_global("UE_VERSION", ue_version)
        .with_global("WITH_CASE_PRESERVING_NAME", case_preserving_flag);

    let module_tree = GospelVMTypeGraphBackend::create_from_module_tree(
        &PathBuf::from("res/gospel/unreal/"),
        &Vec::new(),
        vm_options,
    )?;

    let namespace = TypePtrNamespace {
        type_graph: Arc::new(RwLock::new(module_tree)),
        layout_cache: Arc::new(RwLock::new(TypeLayoutCache::create(target_triplet))),
    };

    let mem = CtxPtr {
        mem,
        fnamepool,
        version: (results.engine_version.major, results.engine_version.minor),
        case_preserving,
    };

    let uobjectarray = <gospel_runtime::static_type_wrappers::Ptr<
        _,
        gospel_bindings::FUObjectArray,
    >>::from_raw_ptr(
        OpaquePtr {
            memory: Arc::new(mem.clone()),
            address: results.guobject_array.0,
        },
        &namespace,
    ).to_ref_checked();

    let mut objects = BTreeMap::<String, ObjectType>::default();
    let mut child_map = HashMap::<String, BTreeSet<String>>::default();

    for i in 0..uobjectarray.num_elements()? {
        let object = uobjectarray.read_item_ptr(i as usize)?;
        let Some(obj) = object else {
            continue;
        };

        let path = obj.path()?;
        // dbg!(&path);

        let obj = read_object(obj, &path);
        // let obj = match obj {
        //     Err(err) => {
        //         eprintln!("{i}: {path} Failed to read: {err}");
        //         continue;
        //     }
        //     r => r,
        // };

        let Some(object) = obj? else {
            continue;
        };

        // println!("{i} {path}");

        // update child_map
        if let Some(outer) = object.get_object().outer.clone() {
            child_map.entry(outer).or_default().insert(path.clone());
        }

        objects.insert(path, object);
    }

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

    let vtables = vtable::analyze_vtables(&mem, &mut objects);

    Ok(ReflectionData {
        image_base_address: image.base_address,
        objects,
        vtables,
    })
}

fn read_object<C: Ctx>(obj: Ref<C, UObject>, path: &str) -> Result<Option<ObjectType>> {
    let class = obj.class_private().read()?.to_ref_checked();

    fn read_props<C: Ctx>(
        ustruct: &Ref<C, UStruct>,
        ptr: &OpaquePtr<C>,
    ) -> Result<OrderMap<String, PropertyValue>> {
        let mut properties = OrderMap::new();
        for prop in ustruct.properties(true) {
            let prop = prop?;
            let array_dim = prop.array_dim().read()? as usize;
            let name = prop.cast_checked::<ZField>().name()?;
            if array_dim == 1 {
                if let Some(value) = read_prop(&prop, ptr.clone(), 0)? {
                    properties.insert(name, value);
                }
            } else {
                let mut elements = vec![];
                let mut success = true;
                for i in 0..array_dim {
                    if let Some(value) = read_prop(&prop, ptr.clone(), i)? {
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
    fn read_prop<C: Ctx>(
        prop: &Ref<C, ZProperty>,
        mut ptr: OpaquePtr<C>,
        index: usize,
    ) -> Result<Option<PropertyValue>> {
        let size = prop.element_size().read()? as u64;
        ptr.address += prop.offset_internal().read()? as u64 + index as u64 * size;
        let f = prop.cast_checked::<ZField>().cast_flags()?;

        let value = if f.contains(EClassCastFlags::CASTCLASS_FStructProperty) {
            let prop = prop.cast_checked::<ZStructProperty>();
            PropertyValue::Struct(read_props(
                &prop
                    .r_struct()
                    .read()?
                    .to_ref_checked()
                    .cast_checked::<UStruct>(),
                &ptr,
            )?)
        // } else if f.contains(EClassCastFlags::CASTCLASS_FStrProperty) {
        //     PropertyValue::Str(ptr.cast::<FString>().read()?)
        // } else if f.contains(EClassCastFlags::CASTCLASS_FNameProperty) {
        //     PropertyValue::Name(ptr.cast::<FName>().read()?)
        // } else if f.contains(EClassCastFlags::CASTCLASS_FTextProperty) {
        //     return Ok(None);
        // } else if f.contains(EClassCastFlags::CASTCLASS_FMulticastInlineDelegateProperty) {
        //     return Ok(None);
        // } else if f.contains(EClassCastFlags::CASTCLASS_FMulticastSparseDelegateProperty) {
        //     return Ok(None);
        // } else if f.contains(EClassCastFlags::CASTCLASS_FMulticastDelegateProperty) {
        //     return Ok(None);
        // } else if f.contains(EClassCastFlags::CASTCLASS_FDelegateProperty) {
        //     return Ok(None);
        // } else if f.contains(EClassCastFlags::CASTCLASS_FBoolProperty) {
        //     let prop = gospel_bindings::FBoolProperty::try_cast(prop)?.unwrap();
        //     let byte_offset = prop.byte_offset_().read()?;
        //     let byte_mask = prop.byte_mask().read()?;
        //     let byte = ptr.byte_offset(byte_offset as usize).cast::<u8>().read()?;
        //     PropertyValue::Bool(byte & byte_mask != 0)
        // } else if f.contains(EClassCastFlags::CASTCLASS_FArrayProperty) {
        //     let prop = gospel_bindings::FArrayProperty::try_cast(prop)?.unwrap();
        //     let array = ptr.cast::<FScriptArray>();

        //     let num = array.num().read()? as usize;
        //     let mut data = Vec::with_capacity(num);
        //     if let Some(data_ptr) = array.data().read()? {
        //         let inner_prop = prop.inner().read()?;
        //         for i in 0..num {
        //             // TODO handle size != alignment
        //             let value = read_prop(&inner_prop, &data_ptr, i)?;
        //             if let Some(value) = value {
        //                 data.push(value);
        //             } else {
        //                 return Ok(None);
        //             }
        //         }
        //     }

        //     PropertyValue::Array(data)
        // } else if f.contains(EClassCastFlags::CASTCLASS_FEnumProperty) {
        //     let prop = gospel_bindings::FEnumProperty::try_cast(prop)?.unwrap();
        //     let underlying = read_prop(&prop.underlying_prop().read()?, &ptr, 0)?
        //         .expect("valid underlying prop");
        //     let value = match underlying {
        //         PropertyValue::Byte(BytePropertyValue::Value(v)) => v as i64,
        //         PropertyValue::Int8(v) => v as i64,
        //         PropertyValue::Int16(v) => v as i64,
        //         PropertyValue::Int(v) => v as i64,
        //         PropertyValue::Int64(v) => v,
        //         PropertyValue::UInt16(v) => v as i64,
        //         PropertyValue::UInt32(v) => v as i64,
        //         e => bail!("underlying enum prop {e:?}"),
        //     };
        //     let names = read_enum(&prop.enum_().read()?.expect("valid enum"))?.names;
        //     let name = names
        //         .into_iter()
        //         .find_map(|(name, v)| (v == value).then_some(name));

        //     PropertyValue::Enum(if let Some(name) = name {
        //         EnumPropertyValue::Name(name)
        //     } else {
        //         EnumPropertyValue::Value(value)
        //     })
        // } else if f.contains(EClassCastFlags::CASTCLASS_FMapProperty) {
        //     // /* offset 0x000 */ Data: TScriptArray<TSizedDefaultAllocator<32> >,
        //     // /* offset 0x010 */ AllocationFlags: TScriptBitArray<FDefaultBitArrayAllocator,void>,
        //     // /* offset 0x030 */ FirstFreeIndex: i32,
        //     // /* offset 0x034 */ NumFreeIndices: i32,

        //     return Ok(None);
        // } else if f.contains(EClassCastFlags::CASTCLASS_FSetProperty) {
        //     //let prop = prop.cast::<FSetProperty>();
        //     //#[derive(Clone, Copy)]
        //     //pub struct FScriptSet;
        //     //impl<C: Clone + StructsTrait> Ptr<FScriptSet, C> {
        //     //    pub fn data(&self) -> Ptr<FScriptArray, C> {
        //     //        self.byte_offset(0).cast()
        //     //    }
        //     //    pub fn allocation_flags(&self) -> Ptr<TBitArray<TInlineAllocator<4>>, C> {
        //     //        self.byte_offset(16).cast()
        //     //    }
        //     //}
        //     //let array = ptr.cast::<FScriptSet>();
        //     //dbg!(array.allocation_flags().read()?);
        //     return Ok(None);
        // } else if f.contains(EClassCastFlags::CASTCLASS_FFloatProperty) {
        //     PropertyValue::Float(ptr.cast::<f32>().read()?.into())
        // } else if f.contains(EClassCastFlags::CASTCLASS_FDoubleProperty) {
        //     PropertyValue::Double(ptr.cast::<f64>().read()?.into())
        // } else if f.contains(EClassCastFlags::CASTCLASS_FByteProperty) {
        //     let prop = gospel_bindings::FByteProperty::try_cast(prop)?.unwrap();
        //     let value = ptr.cast::<u8>().read()?;
        //     PropertyValue::Byte(
        //         if let Some(name) = prop
        //             .enum_()
        //             .read()?
        //             .map(|e| read_enum(&e))
        //             .transpose()?
        //             .and_then(|e| {
        //                 e.names
        //                     .into_iter()
        //                     .find_map(|(name, v)| (v == value as i64).then_some(name))
        //             })
        //         {
        //             BytePropertyValue::Name(name)
        //         } else {
        //             BytePropertyValue::Value(value)
        //         },
        //     )
        // } else if f.contains(EClassCastFlags::CASTCLASS_FUInt16Property) {
        //     PropertyValue::UInt16(ptr.cast::<u16>().read()?)
        // } else if f.contains(EClassCastFlags::CASTCLASS_FUInt32Property) {
        //     PropertyValue::UInt32(ptr.cast::<u32>().read()?)
        // } else if f.contains(EClassCastFlags::CASTCLASS_FUInt64Property) {
        //     PropertyValue::UInt64(ptr.cast::<u64>().read()?)
        // } else if f.contains(EClassCastFlags::CASTCLASS_FInt8Property) {
        //     PropertyValue::Int8(ptr.cast::<i8>().read()?)
        // } else if f.contains(EClassCastFlags::CASTCLASS_FInt16Property) {
        //     PropertyValue::Int16(ptr.cast::<i16>().read()?)
        // } else if f.contains(EClassCastFlags::CASTCLASS_FIntProperty) {
        //     PropertyValue::Int(ptr.cast::<i32>().read()?)
        // } else if f.contains(EClassCastFlags::CASTCLASS_FInt64Property) {
        //     PropertyValue::Int64(ptr.cast::<i64>().read()?)
        // } else if f.contains(EClassCastFlags::CASTCLASS_FObjectProperty) {
        //     let obj = ptr
        //         .cast::<Option<Ptr<UObject, _>>>()
        //         .read()?
        //         .map(|e| e.path())
        //         .transpose()?;
        //     PropertyValue::Object(obj)
        // } else if f.contains(EClassCastFlags::CASTCLASS_FWeakObjectProperty) {
        //     return Ok(None);
        // } else if f.contains(EClassCastFlags::CASTCLASS_FSoftObjectProperty) {
        //     return Ok(None);
        // } else if f.contains(EClassCastFlags::CASTCLASS_FLazyObjectProperty) {
        //     return Ok(None);
        // } else if f.contains(EClassCastFlags::CASTCLASS_FInterfaceProperty) {
        //     return Ok(None);
        // } else if f.contains(EClassCastFlags::CASTCLASS_FFieldPathProperty) {
        //     return Ok(None);
        // } else if f.contains(EClassCastFlags::CASTCLASS_FOptionalProperty) {
        //     return Ok(None);
        } else {
            return Ok(None);
            // unimplemented!("{f:?}");
        };
        Ok(Some(value))
    }

    fn read_object<C: Ctx>(obj: &Ref<C, UObject>) -> Result<Object> {
        let outer = obj
            .outer_private()
            .read()?
            .to_ref()
            .map(|s| s.path())
            .transpose()?;

        let class = obj.class_private().read()?.to_ref_checked();
        let class_name = class.cast_checked::<UObject>().path()?;

        Ok(Object {
            vtable: obj.v_table().read()? as u64,
            object_flags: EObjectFlags::from_bits(obj.object_flags().read()? as u32).unwrap(),
            outer,
            class: class_name,
            children: Default::default(),
            property_values: read_props(
                &class.cast_checked::<UStruct>(),
                &obj.inner_ptr.opaque_ptr,
            )?
            .into(),
        })
    }

    fn read_struct<C: Ctx>(obj: &Ref<C, UStruct>) -> Result<Struct> {
        let mut properties = vec![];
        for prop in obj.properties(false) {
            let prop = prop?;
            let f = prop.cast_checked::<ZField>().cast_flags()?;
            if f.contains(EClassCastFlags::CASTCLASS_FProperty) {
                properties.push(map_prop(&prop.cast_checked::<ZProperty>())?);
            }
        }

        let super_struct = obj
            .super_struct()
            .read()?
            .to_ref()
            .map(|s| s.cast_checked::<UObject>().path())
            .transpose()?;
        Ok(Struct {
            object: read_object(&obj.cast_checked::<UObject>())?,
            super_struct,
            properties,
            properties_size: obj.properties_size().read()? as usize,
            min_alignment: obj.get_min_alignment()? as usize,
        })
    }

    fn read_script_struct<C: Ctx>(obj: &Ref<C, UScriptStruct>) -> Result<ScriptStruct> {
        Ok(ScriptStruct {
            r#struct: read_struct(&obj.cast_checked::<UStruct>())?,
            struct_flags: EStructFlags::from_bits(obj.struct_flags().read()? as i32).unwrap(),
        })
    }

    fn read_class<C: Ctx>(obj: &Ref<C, UClass>) -> Result<Class> {
        let class_flags = EClassFlags::from_bits(obj.class_flags().read()?).unwrap();
        let class_cast_flags = EClassCastFlags::from_bits(obj.class_cast_flags().read()?).unwrap();
        let class_default_object = obj
            .class_default_object()
            .read()?
            .to_ref()
            .map(|s| s.path())
            .transpose()?;
        Ok(Class {
            r#struct: read_struct(&obj.cast_checked::<UStruct>())?,
            class_flags,
            class_cast_flags,
            class_default_object,
            instance_vtable: None,
        })
    }

    // fn read_enum<C: Ctx>(obj: &Ref<C, UEnum>) -> Result<Enum> {
    //     Ok(Enum {
    //         object: read_object(&obj.cast_checked())?,
    //         cpp_type: obj.cpp_type().read()?,
    //         cpp_form: obj.cpp_form().read()?,
    //         enum_flags: (obj.ctx().ue_version() >= (4, 26))
    //             .then(|| obj.enum_flags().read())
    //             .transpose()?,
    //         names: obj.read_names()?,
    //     })
    // }

    if !path.starts_with("/Script/") {
        return Ok(None);
    }
    let f = EClassCastFlags::from_bits(class.class_cast_flags().read()?).unwrap();
    let object = if f.contains(EClassCastFlags::CASTCLASS_UClass) {
        ObjectType::Class(read_class(&obj.cast_checked())?)
    } else if f.contains(EClassCastFlags::CASTCLASS_UFunction) {
        let full_obj = obj.cast_checked::<UFunction>();
        let function_flags =
            EFunctionFlags::from_bits(full_obj.function_flags().cast_checked::<u32>().read()?)
                .unwrap();
        ObjectType::Function(Function {
            r#struct: read_struct(&obj.cast_checked())?,
            function_flags,
            func: full_obj.func().read()?.inner_ptr.opaque_ptr.address,
        })
    } else if f.contains(EClassCastFlags::CASTCLASS_UScriptStruct) {
        ObjectType::ScriptStruct(read_script_struct(&obj.cast_checked::<UScriptStruct>())?)
    // } else if f.contains(EClassCastFlags::CASTCLASS_UEnum) {
    //     ObjectType::Enum(read_enum(&obj.cast_checked())?)
    } else if f.contains(EClassCastFlags::CASTCLASS_UPackage) {
        ObjectType::Package(Package {
            object: read_object(&obj)?,
        })
    } else {
        ObjectType::Object(read_object(&obj)?)
        //println!("{path:?} {:?}", f);
    };
    Ok(Some(object))
}
