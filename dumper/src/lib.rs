mod containers;
mod header;
mod mem;
mod objects;
pub mod structs;
mod vtable;

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result, bail};
use containers::{FName, FString};
use mem::{Ctx, Mem, MemCache, NameTrait, Ptr, StructsTrait};
use objects::FOptionalProperty;
use ordermap::OrderMap;
use patternsleuth_image::image::Image;
use patternsleuth_resolvers::{impl_try_collector, resolve};
use read_process_memory::{Pid, ProcessHandle};
use ue_reflection::{
    BytePropertyValue, Class, EClassCastFlags, Enum, EnumPropertyValue, Function, Object,
    ObjectType, Package, Property, PropertyType, PropertyValue, ReflectionData, ScriptStruct,
    Struct,
};

use crate::containers::PtrFNamePool;
use crate::mem::VersionTrait;
use crate::objects::{
    FUObjectArray, UClass, UEnum, UFunction, UObject, UScriptStruct, UStruct, ZArrayProperty,
    ZBoolProperty, ZByteProperty, ZClassProperty, ZDelegateProperty, ZEnumProperty,
    ZInterfaceProperty, ZLazyObjectProperty, ZMapProperty, ZMulticastDelegateProperty,
    ZObjectProperty, ZProperty, ZSetProperty, ZSoftClassProperty, ZSoftObjectProperty,
    ZStructProperty, ZWeakObjectProperty,
};
use crate::structs::Structs;

impl_try_collector! {
    #[derive(Debug, PartialEq, Clone)]
    struct Resolution {
        guobject_array: patternsleuth_resolvers::unreal::guobject_array::GUObjectArray,
        fname_pool: patternsleuth_resolvers::unreal::fname::FNamePool,
        engine_version: patternsleuth_resolvers::unreal::engine_version::EngineVersion,
    }
}

// TODO
// [ ] UStruct?
// [ ] interfaces
// [ ] functions signatures
// [ ] native function pointers
// [ ] dynamic structs
// [ ] ue version info

trait MemComplete: Mem + Clone + NameTrait + StructsTrait + VersionTrait {}
impl<T: Mem + Clone + NameTrait + StructsTrait + VersionTrait> MemComplete for T {}

fn read_path<M: MemComplete>(obj: &Ptr<UObject, M>) -> Result<String> {
    let mut objects = vec![obj.clone()];

    let mut obj = obj.clone();
    while let Some(outer) = obj.outer_private().read()? {
        objects.push(outer.clone());
        obj = outer;
    }

    let mut path = String::new();
    let mut prev: Option<&Ptr<UObject, M>> = None;
    for obj in objects.iter().rev() {
        if let Some(prev) = prev {
            let sep = if prev
                .class_private()
                .read()?
                .class_cast_flags()
                .read()?
                .contains(EClassCastFlags::CASTCLASS_UPackage)
            {
                '.'
            } else {
                ':'
            };
            path.push(sep);
        }
        path.push_str(&obj.name_private().read()?);
        prev = Some(obj);
    }

    Ok(path)
}

fn map_prop<M: MemComplete>(ptr: &Ptr<ZProperty, M>) -> Result<Property> {
    let name = ptr.zfield().name_private().read()?;
    let f = ptr.zfield().cast_flags()?;

    let t = if f.contains(EClassCastFlags::CASTCLASS_FStructProperty) {
        let prop = ptr.cast::<ZStructProperty>();
        let s = prop.struct_().read()?.path()?;
        PropertyType::Struct { r#struct: s }
    } else if f.contains(EClassCastFlags::CASTCLASS_FStrProperty) {
        PropertyType::Str
    } else if f.contains(EClassCastFlags::CASTCLASS_FNameProperty) {
        PropertyType::Name
    } else if f.contains(EClassCastFlags::CASTCLASS_FTextProperty) {
        PropertyType::Text
    } else if f.contains(EClassCastFlags::CASTCLASS_FMulticastInlineDelegateProperty) {
        let prop = ptr.cast::<ZMulticastDelegateProperty>();
        let signature_function = prop
            .signature_function()
            .read()?
            .map(|e| e.path())
            .transpose()?;
        PropertyType::MulticastInlineDelegate { signature_function }
    } else if f.contains(EClassCastFlags::CASTCLASS_FMulticastSparseDelegateProperty) {
        let prop = ptr.cast::<ZMulticastDelegateProperty>();
        let signature_function = prop
            .signature_function()
            .read()?
            .map(|e| e.path())
            .transpose()?;
        PropertyType::MulticastSparseDelegate { signature_function }
    } else if f.contains(EClassCastFlags::CASTCLASS_FMulticastDelegateProperty) {
        let prop = ptr.cast::<ZMulticastDelegateProperty>();
        let signature_function = prop
            .signature_function()
            .read()?
            .map(|e| e.path())
            .transpose()?;
        PropertyType::MulticastDelegate { signature_function }
    } else if f.contains(EClassCastFlags::CASTCLASS_FDelegateProperty) {
        let prop = ptr.cast::<ZDelegateProperty>();
        let signature_function = prop
            .signature_function()
            .read()?
            .map(|e| e.path())
            .transpose()?;
        PropertyType::Delegate { signature_function }
    } else if f.contains(EClassCastFlags::CASTCLASS_FBoolProperty) {
        let prop = ptr.cast::<ZBoolProperty>();
        PropertyType::Bool {
            field_size: prop.field_size().read()?,
            byte_offset: prop.byte_offset_().read()?,
            byte_mask: prop.byte_mask().read()?,
            field_mask: prop.field_mask().read()?,
        }
    } else if f.contains(EClassCastFlags::CASTCLASS_FArrayProperty) {
        let prop = ptr.cast::<ZArrayProperty>();
        PropertyType::Array {
            inner: map_prop(&prop.inner().read()?.cast())?.into(),
        }
    } else if f.contains(EClassCastFlags::CASTCLASS_FEnumProperty) {
        let prop = ptr.cast::<ZEnumProperty>();
        PropertyType::Enum {
            container: map_prop(&prop.underlying_prop().read()?.cast())?.into(),
            r#enum: prop.enum_().read()?.map(|e| e.path()).transpose()?,
        }
    } else if f.contains(EClassCastFlags::CASTCLASS_FMapProperty) {
        let prop = ptr.cast::<ZMapProperty>();
        PropertyType::Map {
            key_prop: map_prop(&prop.key_prop().read()?.cast())?.into(),
            value_prop: map_prop(&prop.value_prop().read()?.cast())?.into(),
        }
    } else if f.contains(EClassCastFlags::CASTCLASS_FSetProperty) {
        let prop = ptr.cast::<ZSetProperty>();
        PropertyType::Set {
            key_prop: map_prop(&prop.element_prop().read()?.cast())?.into(),
        }
    } else if f.contains(EClassCastFlags::CASTCLASS_FFloatProperty) {
        PropertyType::Float
    } else if f.contains(EClassCastFlags::CASTCLASS_FDoubleProperty) {
        PropertyType::Double
    } else if f.contains(EClassCastFlags::CASTCLASS_FByteProperty) {
        let prop = ptr.cast::<ZByteProperty>();
        PropertyType::Byte {
            r#enum: prop.enum_().read()?.map(|e| e.path()).transpose()?,
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
        let property_class = prop.fobject_property().property_class().read()?.path()?;
        let meta_class = prop.meta_class().read()?.path()?;
        PropertyType::Class {
            property_class,
            meta_class,
        }
    } else if f.contains(EClassCastFlags::CASTCLASS_FObjectProperty) {
        let prop = ptr.cast::<ZObjectProperty>();
        let property_class = prop.property_class().read()?.path()?;
        PropertyType::Object { property_class }
    } else if f.contains(EClassCastFlags::CASTCLASS_FSoftClassProperty) {
        let prop = ptr.cast::<ZSoftClassProperty>();
        let property_class = prop
            .fsoft_object_property()
            .property_class()
            .read()?
            .path()?;
        let meta_class = prop.meta_class().read()?.path()?;
        PropertyType::SoftClass {
            property_class,
            meta_class,
        }
    } else if f.contains(EClassCastFlags::CASTCLASS_FSoftObjectProperty) {
        let prop = ptr.cast::<ZSoftObjectProperty>();
        let property_class = prop.property_class().read()?.path()?;
        PropertyType::SoftObject { property_class }
    } else if f.contains(EClassCastFlags::CASTCLASS_FWeakObjectProperty) {
        let prop = ptr.cast::<ZWeakObjectProperty>();
        let c = prop.property_class().read()?.path()?;
        PropertyType::WeakObject { property_class: c }
    } else if f.contains(EClassCastFlags::CASTCLASS_FLazyObjectProperty) {
        let prop = ptr.cast::<ZLazyObjectProperty>();
        let c = prop.property_class().read()?.path()?;
        PropertyType::LazyObject { property_class: c }
    } else if f.contains(EClassCastFlags::CASTCLASS_FInterfaceProperty) {
        let prop = ptr.cast::<ZInterfaceProperty>();
        let interface_class = prop.interface_class().read()?.path()?;
        PropertyType::Interface { interface_class }
    } else if f.contains(EClassCastFlags::CASTCLASS_FFieldPathProperty) {
        // TODO
        PropertyType::FieldPath
    } else if f.contains(EClassCastFlags::CASTCLASS_FOptionalProperty) {
        let prop = ptr.cast::<FOptionalProperty>();
        PropertyType::Optional {
            inner: map_prop(&prop.value_property().read()?.cast())?.into(),
        }
    } else {
        unimplemented!("{f:?}");
    };

    let prop = ptr.cast::<ZProperty>();
    Ok(Property {
        name,
        offset: prop.offset_internal().read()? as usize,
        array_dim: prop.array_dim().read()? as usize,
        size: prop.element_size().read()? as usize,
        flags: prop.property_flags().read()?,
        r#type: t,
    })
}

#[derive(Clone)]
struct ImgMem<'img, 'data>(&'img Image<'data>);

impl Mem for ImgMem<'_, '_> {
    fn read_buf(&self, address: usize, buf: &mut [u8]) -> Result<()> {
        self.0.memory.read(address, buf)?;
        Ok(())
    }
}

pub enum Input {
    Process(i32),
    Dump(PathBuf),
}

pub fn dump(input: Input, struct_info: Option<Structs>) -> Result<ReflectionData> {
    match input {
        Input::Process(pid) => {
            let handle: ProcessHandle = (pid as Pid).try_into()?;
            let mem = MemCache::wrap(handle);
            let image = patternsleuth_image::process::external::read_image_from_pid(pid)?;
            dump_inner(mem, &image, struct_info)
        }
        Input::Dump(path) => {
            let file = std::fs::File::open(path)?;
            let mmap = unsafe { memmap2::MmapOptions::new().map(&file)? };

            let image = patternsleuth_image::image::Image::read::<&str>(None, &mmap, None, false)?;
            let mem = ImgMem(&image);
            dump_inner(mem, &image, struct_info)
        }
    }
}

use script_containers::*;
mod script_containers {
    use super::*;

    #[derive(Clone, Copy)]
    pub struct FScriptArray;
    impl<C: Clone + StructsTrait> Ptr<FScriptArray, C> {
        pub fn data(&self) -> Ptr<Option<Ptr<(), C>>, C> {
            self.byte_offset(0).cast()
        }
        pub fn num(&self) -> Ptr<u32, C> {
            self.byte_offset(8).cast()
        }
    }
}

fn dump_inner<M: Mem + Clone>(
    mem: M,
    image: &Image<'_>,
    struct_info: Option<Structs>,
) -> Result<ReflectionData> {
    let results = resolve(image, Resolution::resolver())?;
    println!("{results:X?}");

    let fnamepool = PtrFNamePool(results.fname_pool.0);

    let case_preserving = false;

    let struct_info = if let Some(provided_info) = struct_info {
        provided_info
    } else {
        structs::get_struct_info_for_version(&results.engine_version, case_preserving)
            .with_context(|| {
                format!(
                    "Failed to compute struct offsets via Gospel for {:?}",
                    results.engine_version
                )
            })?
    };

    let mem = Ctx {
        mem,
        fnamepool,
        structs: Arc::new(
            struct_info
                .0
                .into_iter()
                .map(|s| (s.name.clone(), s))
                .collect(),
        ),
        version: (results.engine_version.major, results.engine_version.minor),
        case_preserving,
    };

    let uobjectarray = Ptr::<FUObjectArray, _>::new(results.guobject_array.0, mem);

    let mut objects = BTreeMap::<String, ObjectType>::default();
    let mut child_map = HashMap::<String, BTreeSet<String>>::default();

    for i in 0..uobjectarray.num_elements()? {
        let obj_item = uobjectarray.read_item_ptr(i as usize)?;
        let Some(obj) = obj_item.object().read()? else {
            continue;
        };

        let path = obj.path()?;

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

    let vtables = vtable::analyze_vtables(image, &mut objects);

    Ok(ReflectionData {
        image_base_address: image.base_address as u64,
        objects,
        vtables,
    })
}

fn read_object<M: Mem + Clone>(
    obj: Ptr<UObject, Ctx<M>>,
    path: &str,
) -> Result<Option<ObjectType>> {
    let class = obj.class_private().read()?;

    fn read_props<M: MemComplete>(
        ustruct: &Ptr<UStruct, M>,
        ptr: &Ptr<(), M>,
    ) -> Result<OrderMap<String, PropertyValue>> {
        let mut properties = OrderMap::new();
        for prop in ustruct.properties(true) {
            let prop = prop?;
            let array_dim = prop.array_dim().read()? as usize;
            let name = prop.zfield().name_private().read()?;
            if array_dim == 1 {
                if let Some(value) = read_prop(&prop, ptr, 0)? {
                    properties.insert(name, value);
                }
            } else {
                let mut elements = vec![];
                let mut success = true;
                for i in 0..array_dim {
                    if let Some(value) = read_prop(&prop, ptr, i)? {
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
    fn read_prop<M: MemComplete>(
        prop: &Ptr<ZProperty, M>,
        ptr: &Ptr<(), M>,
        index: usize,
    ) -> Result<Option<PropertyValue>> {
        let size = prop.element_size().read()? as usize;
        let ptr = ptr.byte_offset(prop.offset_internal().read()? as usize + index * size);
        let f = prop.zfield().cast_flags()?;

        let value = if f.contains(EClassCastFlags::CASTCLASS_FStructProperty) {
            let prop = prop.cast::<ZStructProperty>();
            PropertyValue::Struct(read_props(&prop.struct_().read()?.ustruct(), &ptr)?)
        } else if f.contains(EClassCastFlags::CASTCLASS_FStrProperty) {
            PropertyValue::Str(ptr.cast::<FString>().read()?)
        } else if f.contains(EClassCastFlags::CASTCLASS_FNameProperty) {
            PropertyValue::Name(ptr.cast::<FName>().read()?)
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
            let byte_offset = prop.byte_offset_().read()?;
            let byte_mask = prop.byte_mask().read()?;
            let byte = ptr.byte_offset(byte_offset as usize).cast::<u8>().read()?;
            PropertyValue::Bool(byte & byte_mask != 0)
        } else if f.contains(EClassCastFlags::CASTCLASS_FArrayProperty) {
            let prop = prop.cast::<ZArrayProperty>();
            let array = ptr.cast::<FScriptArray>();

            let num = array.num().read()? as usize;
            let mut data = Vec::with_capacity(num);
            if let Some(data_ptr) = array.data().read()? {
                let inner_prop = prop.inner().read()?;
                for i in 0..num {
                    // TODO handle size != alignment
                    let value = read_prop(&inner_prop, &data_ptr, i)?;
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
            let underlying = read_prop(&prop.underlying_prop().read()?, &ptr, 0)?
                .expect("valid underlying prop");
            let value = match underlying {
                PropertyValue::Byte(BytePropertyValue::Value(v)) => v as i64,
                PropertyValue::Int8(v) => v as i64,
                PropertyValue::Int16(v) => v as i64,
                PropertyValue::Int(v) => v as i64,
                PropertyValue::Int64(v) => v,
                PropertyValue::UInt16(v) => v as i64,
                PropertyValue::UInt32(v) => v as i64,
                e => bail!("underlying enum prop {e:?}"),
            };
            let names = read_enum(&prop.enum_().read()?.expect("valid enum"))?.names;
            let name = names
                .into_iter()
                .find_map(|(name, v)| (v == value).then_some(name));

            PropertyValue::Enum(if let Some(name) = name {
                EnumPropertyValue::Name(name)
            } else {
                EnumPropertyValue::Value(value)
            })
        } else if f.contains(EClassCastFlags::CASTCLASS_FMapProperty) {
            // /* offset 0x000 */ Data: TScriptArray<TSizedDefaultAllocator<32> >,
            // /* offset 0x010 */ AllocationFlags: TScriptBitArray<FDefaultBitArrayAllocator,void>,
            // /* offset 0x030 */ FirstFreeIndex: i32,
            // /* offset 0x034 */ NumFreeIndices: i32,

            return Ok(None);
        } else if f.contains(EClassCastFlags::CASTCLASS_FSetProperty) {
            //let prop = prop.cast::<FSetProperty>();
            //#[derive(Clone, Copy)]
            //pub struct FScriptSet;
            //impl<C: Clone + StructsTrait> Ptr<FScriptSet, C> {
            //    pub fn data(&self) -> Ptr<FScriptArray, C> {
            //        self.byte_offset(0).cast()
            //    }
            //    pub fn allocation_flags(&self) -> Ptr<TBitArray<TInlineAllocator<4>>, C> {
            //        self.byte_offset(16).cast()
            //    }
            //}
            //let array = ptr.cast::<FScriptSet>();
            //dbg!(array.allocation_flags().read()?);
            return Ok(None);
        } else if f.contains(EClassCastFlags::CASTCLASS_FFloatProperty) {
            PropertyValue::Float(ptr.cast::<f32>().read()?.into())
        } else if f.contains(EClassCastFlags::CASTCLASS_FDoubleProperty) {
            PropertyValue::Double(ptr.cast::<f64>().read()?.into())
        } else if f.contains(EClassCastFlags::CASTCLASS_FByteProperty) {
            let prop = prop.cast::<ZByteProperty>();
            let value = ptr.cast::<u8>().read()?;
            PropertyValue::Byte(
                if let Some(name) = prop
                    .enum_()
                    .read()?
                    .map(|e| read_enum(&e))
                    .transpose()?
                    .and_then(|e| {
                        e.names
                            .into_iter()
                            .find_map(|(name, v)| (v == value as i64).then_some(name))
                    })
                {
                    BytePropertyValue::Name(name)
                } else {
                    BytePropertyValue::Value(value)
                },
            )
        } else if f.contains(EClassCastFlags::CASTCLASS_FUInt16Property) {
            PropertyValue::UInt16(ptr.cast::<u16>().read()?)
        } else if f.contains(EClassCastFlags::CASTCLASS_FUInt32Property) {
            PropertyValue::UInt32(ptr.cast::<u32>().read()?)
        } else if f.contains(EClassCastFlags::CASTCLASS_FUInt64Property) {
            PropertyValue::UInt64(ptr.cast::<u64>().read()?)
        } else if f.contains(EClassCastFlags::CASTCLASS_FInt8Property) {
            PropertyValue::Int8(ptr.cast::<i8>().read()?)
        } else if f.contains(EClassCastFlags::CASTCLASS_FInt16Property) {
            PropertyValue::Int16(ptr.cast::<i16>().read()?)
        } else if f.contains(EClassCastFlags::CASTCLASS_FIntProperty) {
            PropertyValue::Int(ptr.cast::<i32>().read()?)
        } else if f.contains(EClassCastFlags::CASTCLASS_FInt64Property) {
            PropertyValue::Int64(ptr.cast::<i64>().read()?)
        } else if f.contains(EClassCastFlags::CASTCLASS_FObjectProperty) {
            let obj = ptr
                .cast::<Option<Ptr<UObject, _>>>()
                .read()?
                .map(|e| e.path())
                .transpose()?;
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
        } else {
            unimplemented!("{f:?}");
        };
        Ok(Some(value))
    }

    fn read_object<M: MemComplete>(obj: &Ptr<UObject, M>) -> Result<Object> {
        let outer = obj.outer_private().read()?.map(|s| s.path()).transpose()?;

        let class = obj.class_private().read()?;
        let class_name = class.path()?;

        Ok(Object {
            vtable: obj.vtable().read()? as u64,
            object_flags: obj.object_flags().read()?,
            outer,
            class: class_name,
            children: Default::default(),
            property_values: read_props(&class.ustruct(), &obj.cast())?.into(),
        })
    }

    fn read_struct<M: MemComplete>(obj: &Ptr<UStruct, M>) -> Result<Struct> {
        let mut properties = vec![];
        for prop in obj.properties(false) {
            let prop = prop?;
            let f = prop.zfield().cast_flags()?;
            if f.contains(EClassCastFlags::CASTCLASS_FProperty) {
                properties.push(map_prop(&prop.cast::<ZProperty>())?);
            }
        }

        let super_struct = obj.super_struct().read()?.map(|s| s.path()).transpose()?;
        Ok(Struct {
            object: read_object(&obj.cast())?,
            super_struct,
            properties,
            properties_size: obj.properties_size().read()? as usize,
            min_alignment: obj.min_alignment().read()? as usize,
        })
    }

    fn read_script_struct<M: MemComplete>(obj: &Ptr<UScriptStruct, M>) -> Result<ScriptStruct> {
        Ok(ScriptStruct {
            r#struct: read_struct(&obj.ustruct())?,
            struct_flags: obj.struct_flags().read()?,
        })
    }

    fn read_class<M: MemComplete>(obj: &Ptr<UClass, M>) -> Result<Class> {
        let class_flags = obj.class_flags().read()?;
        let class_cast_flags = obj.class_cast_flags().read()?;
        let class_default_object = obj
            .class_default_object()
            .read()?
            .map(|s| s.path())
            .transpose()?;
        Ok(Class {
            r#struct: read_struct(&obj.cast())?,
            class_flags,
            class_cast_flags,
            class_default_object,
            instance_vtable: None,
        })
    }

    fn read_enum<M: MemComplete>(obj: &Ptr<UEnum, M>) -> Result<Enum> {
        Ok(Enum {
            object: read_object(&obj.cast())?,
            cpp_type: obj.cpp_type().read()?,
            cpp_form: obj.cpp_form().read()?,
            enum_flags: (obj.ctx().ue_version() >= (4, 26))
                .then(|| obj.enum_flags().read())
                .transpose()?,
            names: obj.read_names()?,
        })
    }

    if !path.starts_with("/Script/") {
        return Ok(None);
    }
    let f = class.class_cast_flags().read()?;
    let object = if f.contains(EClassCastFlags::CASTCLASS_UClass) {
        ObjectType::Class(read_class(&obj.cast())?)
    } else if f.contains(EClassCastFlags::CASTCLASS_UFunction) {
        let full_obj = obj.cast::<UFunction>();
        let function_flags = full_obj.function_flags().read()?;
        ObjectType::Function(Function {
            r#struct: read_struct(&obj.cast())?,
            function_flags,
            func: full_obj.func().read()? as u64,
        })
    } else if f.contains(EClassCastFlags::CASTCLASS_UScriptStruct) {
        ObjectType::ScriptStruct(read_script_struct(&obj.cast())?)
    } else if f.contains(EClassCastFlags::CASTCLASS_UEnum) {
        ObjectType::Enum(read_enum(&obj.cast())?)
    } else if f.contains(EClassCastFlags::CASTCLASS_UPackage) {
        ObjectType::Package(Package {
            object: read_object(&obj)?,
        })
    } else {
        let obj = obj.cast::<UObject>();
        ObjectType::Object(read_object(&obj)?)
        //println!("{path:?} {:?}", f);
    };
    Ok(Some(object))
}
