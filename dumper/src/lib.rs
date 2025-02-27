mod containers;
mod mem;
mod objects;

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use mem::{Ctx, CtxPtr, ExternalPtr, Mem, MemCache, NameTrait, StructsTrait};
use patternsleuth_image::image::Image;
use patternsleuth_resolvers::{impl_try_collector, resolve};
use read_process_memory::{Pid, ProcessHandle};
use serde::{Deserialize, Serialize};
use ue_reflection::{
    Class, EClassCastFlags, EClassFlags, EFunctionFlags, EStructFlags, Enum, Function, Object,
    ObjectType, Property, PropertyType, Struct,
};

use crate::containers::PtrFNamePool;
use crate::objects::{
    FArrayProperty, FBoolProperty, FByteProperty, FEnumProperty, FField, FInterfaceProperty,
    FLazyObjectProperty, FMapProperty, FObjectProperty, FProperty, FSetProperty,
    FSoftObjectProperty, FStructProperty, FUObjectArray, FWeakObjectProperty, UClass, UEnum,
    UFunction, UObject, UScriptStruct, UStruct,
};

impl_try_collector! {
    #[derive(Debug, PartialEq, Clone)]
    struct DrgResolution {
        guobject_array: patternsleuth_resolvers::unreal::guobject_array::GUObjectArray,
        fname_pool: patternsleuth_resolvers::unreal::fname::FNamePool,
    }
}

// TODO
// [ ] UStruct?
// [ ] interfaces
// [ ] functions signatures
// [ ] native function pointers
// [ ] dynamic structs
// [ ] ue version info

trait MemComplete: Mem + Clone + NameTrait + StructsTrait {}
impl<T: Mem + Clone + NameTrait + StructsTrait> MemComplete for T {}

fn read_path<M: MemComplete>(obj: &CtxPtr<UObject, M>) -> Result<String> {
    let mut components = vec![];
    let name = obj.name_private().read()?;
    components.push(name);

    let mut obj = obj.clone();
    while let Some(outer) = obj.outer_private().read()? {
        let name = outer.name_private().read()?;
        components.push(name);
        obj = outer;
    }
    components.reverse();
    Ok(components.join("."))
}

fn map_prop<M: MemComplete>(ptr: &CtxPtr<FProperty, M>) -> Result<Property> {
    let name = ptr.ffield().name_private().read()?;
    let field_class = ptr.ffield().class_private().read()?;
    let f = field_class.cast_flags().read()?;

    dbg!(f);
    let t = if f.contains(EClassCastFlags::CASTCLASS_FStructProperty) {
        let prop = ptr.cast::<FStructProperty>();
        let s = read_path(&prop.struct_().read()?.ustruct().ufield().uobject())?;
        PropertyType::Struct { r#struct: s }
    } else if f.contains(EClassCastFlags::CASTCLASS_FStrProperty) {
        PropertyType::Str
    } else if f.contains(EClassCastFlags::CASTCLASS_FNameProperty) {
        PropertyType::Name
    } else if f.contains(EClassCastFlags::CASTCLASS_FTextProperty) {
        PropertyType::Text
    } else if f.contains(EClassCastFlags::CASTCLASS_FMulticastInlineDelegateProperty) {
        // TODO function signature
        PropertyType::MulticastInlineDelegate
    } else if f.contains(EClassCastFlags::CASTCLASS_FMulticastSparseDelegateProperty) {
        // TODO function signature
        PropertyType::MulticastSparseDelegate
    } else if f.contains(EClassCastFlags::CASTCLASS_FDelegateProperty) {
        // TODO function signature
        PropertyType::Delegate
    } else if f.contains(EClassCastFlags::CASTCLASS_FBoolProperty) {
        let prop = ptr.cast::<FBoolProperty>();
        PropertyType::Bool {
            field_size: prop.field_size().read()?,
            byte_offset: prop.byte_offset_().read()?,
            byte_mask: prop.byte_mask().read()?,
            field_mask: prop.field_mask().read()?,
        }
    } else if f.contains(EClassCastFlags::CASTCLASS_FArrayProperty) {
        let prop = ptr.cast::<FArrayProperty>();
        PropertyType::Array {
            inner: map_prop(&prop.inner().read()?.cast())?.r#type.into(),
        }
    } else if f.contains(EClassCastFlags::CASTCLASS_FEnumProperty) {
        let prop = ptr.cast::<FEnumProperty>();
        PropertyType::Enum {
            container: map_prop(&prop.underlying_prop().read()?.cast())?
                .r#type
                .into(),
            r#enum: prop
                .enum_()
                .read()?
                .map(|e| read_path(&e.ufield().uobject()))
                .transpose()?,
        }
    } else if f.contains(EClassCastFlags::CASTCLASS_FMapProperty) {
        let prop = ptr.cast::<FMapProperty>();
        PropertyType::Map {
            key_prop: map_prop(&prop.key_prop().read()?.cast())?.r#type.into(),
            value_prop: map_prop(&prop.value_prop().read()?.cast())?.r#type.into(),
        }
    } else if f.contains(EClassCastFlags::CASTCLASS_FSetProperty) {
        let prop = ptr.cast::<FSetProperty>();
        PropertyType::Set {
            key_prop: map_prop(&prop.element_prop().read()?.cast())?.r#type.into(),
        }
    } else if f.contains(EClassCastFlags::CASTCLASS_FFloatProperty) {
        PropertyType::Float
    } else if f.contains(EClassCastFlags::CASTCLASS_FDoubleProperty) {
        PropertyType::Double
    } else if f.contains(EClassCastFlags::CASTCLASS_FByteProperty) {
        let prop = ptr.cast::<FByteProperty>();
        PropertyType::Byte {
            r#enum: prop
                .enum_()
                .read()?
                .map(|e| read_path(&e.ufield().uobject()))
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
    } else if f.contains(EClassCastFlags::CASTCLASS_FObjectProperty) {
        let prop = ptr.cast::<FObjectProperty>();
        //dbg!(&prop.property_class());
        //dbg!(&prop.property_class().read()?);
        let class = prop
            .property_class()
            .read()?
            .map(|c| read_path(&c.ustruct().ufield().uobject()))
            .transpose()?;

        //let c = read_path(&prop.property_class().read()?.ustruct().ufield().uobject())?;
        PropertyType::Object { class }
    } else if f.contains(EClassCastFlags::CASTCLASS_FWeakObjectProperty) {
        let prop = ptr.cast::<FWeakObjectProperty>();
        let c = read_path(&prop.property_class().read()?.ustruct().ufield().uobject())?;
        PropertyType::WeakObject { class: c }
    } else if f.contains(EClassCastFlags::CASTCLASS_FSoftObjectProperty) {
        let prop = ptr.cast::<FSoftObjectProperty>();
        let c = read_path(&prop.property_class().read()?.ustruct().ufield().uobject())?;
        PropertyType::SoftObject { class: c }
    } else if f.contains(EClassCastFlags::CASTCLASS_FLazyObjectProperty) {
        let prop = ptr.cast::<FLazyObjectProperty>();
        let c = read_path(&prop.property_class().read()?.ustruct().ufield().uobject())?;
        PropertyType::LazyObject { class: c }
    } else if f.contains(EClassCastFlags::CASTCLASS_FInterfaceProperty) {
        let prop = ptr.cast::<FInterfaceProperty>();
        let c = read_path(&prop.interface_class().read()?.ustruct().ufield().uobject())?;
        PropertyType::Interface { class: c }
    } else if f.contains(EClassCastFlags::CASTCLASS_FFieldPathProperty) {
        // TODO
        PropertyType::FieldPath
    } else if f.contains(EClassCastFlags::CASTCLASS_FOptionalProperty) {
        // TODO
        PropertyType::Optional
    } else {
        unimplemented!("{f:?}");
    };

    let prop = ptr.cast::<FProperty>();
    Ok(Property {
        name,
        offset: prop.offset_internal().read()? as usize,
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

#[derive(Serialize, Deserialize)]
struct StructMember {
    name: String,
    offset: u64,
    size: u64,
    //type_name: String,
}

#[derive(Serialize, Deserialize)]
struct StructInfo {
    name: String,
    size: u64,
    members: Vec<StructMember>,
}

enum Input {
    Process(i32),
    Dump(PathBuf),
}

pub fn dump(input: Input) -> Result<()> {
    match input {
        Input::Process(pid) => {
            let handle: ProcessHandle = (pid as Pid).try_into()?;
            let mem = MemCache::wrap(handle);
            let image = patternsleuth_image::process::external::read_image_from_pid(pid)?;
            dump_inner(mem, &image)?;
        }
        Input::Dump(path) => {
            let data = std::fs::read(path)?;
            let image = patternsleuth_image::image::Image::read::<&str>(None, &data, None, false)?;
            let mem = ImgMem(&image);
            dump_inner(mem, &image)?;
        }
    };
    Ok(())
}

fn dump_inner<M: Mem + Clone>(mem: M, image: &Image<'_>) -> Result<()> {
    let results = resolve(&image, DrgResolution::resolver())?;

    let guobjectarray = ExternalPtr::<FUObjectArray>::new(results.guobject_array.0);
    let fnamepool = PtrFNamePool(results.fname_pool.0);

    println!("GUObjectArray = {guobjectarray:x?} FNamePool = {fnamepool:x?}");

    let structs: Vec<StructInfo> = serde_json::from_slice(&std::fs::read("../struct_info.json")?)?;

    let mem = Ctx {
        mem,
        fnamepool,
        structs: Arc::new(structs.into_iter().map(|s| (s.name.clone(), s)).collect()),
    };

    let uobject_array = guobjectarray.ctx(mem);

    let mut objects = BTreeMap::<String, ObjectType>::default();

    for i in 0..uobject_array.obj_object().num_elements().read()? {
        let obj_item = uobject_array.obj_object().read_item_ptr(i as usize)?;
        let Some(obj) = obj_item.object().read()? else {
            continue;
        };
        let class = obj.class_private().read()?;

        let path = read_path(&obj)?;

        fn read_object<M: MemComplete>(obj: &CtxPtr<UObject, M>) -> Result<Object> {
            let outer = obj
                .outer_private()
                .read()?
                .map(|s| read_path(&s))
                .transpose()?;
            let class = obj
                .outer_private()
                .read()?
                .map(|s| read_path(&s))
                .transpose()?;
            Ok(Object { outer, class })
        }

        fn read_struct<M: MemComplete>(obj: &CtxPtr<UStruct, M>) -> Result<Struct> {
            let mut properties = vec![];
            let mut field = obj.child_properties();
            while let Some(next) = field.read()? {
                let f = next.class_private().read()?.cast_flags().read()?;
                if f.contains(EClassCastFlags::CASTCLASS_FProperty) {
                    properties.push(map_prop(&next.cast::<FProperty>())?);
                }

                field = next.next();
            }
            let super_struct = obj
                .super_struct()
                .read()?
                .map(|s| read_path(&s.ufield().uobject()))
                .transpose()?;
            Ok(Struct {
                object: read_object(&obj.cast())?,
                super_struct,
                properties,
            })
        }

        let f = class.class_cast_flags().read()?;
        if f.contains(EClassCastFlags::CASTCLASS_UClass) {
            dbg!(&path);
            let obj = obj.cast::<UClass>();
            let flags = obj.class_flags();
            //if flags.read()?.contains(EClassFlags::CLASS_Native) {
            let class_default_object = obj
                .class_default_object()
                .read()?
                .map(|s| read_path(&s))
                .transpose()?;
            objects.insert(
                path,
                ObjectType::Class(Class {
                    r#struct: read_struct(&obj.cast())?,
                    class_default_object,
                }),
            );
            //}
        } else if f.contains(EClassCastFlags::CASTCLASS_UFunction) {
            let flags = obj.cast::<UFunction>().function_flags();
            //if flags.read()?.contains(EFunctionFlags::FUNC_Native) {
            objects.insert(
                path,
                ObjectType::Function(Function {
                    r#struct: read_struct(&obj.cast())?,
                }),
            );
            //}
        } else if f.contains(EClassCastFlags::CASTCLASS_UScriptStruct) {
            let flags = obj.cast::<UScriptStruct>().struct_flags();
            //if flags.read()?.contains(EStructFlags::STRUCT_Native) {
            objects.insert(path, ObjectType::Struct(read_struct(&obj.cast())?));
            //}
        } else if f.contains(EClassCastFlags::CASTCLASS_UEnum) {
            let full_obj = obj.cast::<UEnum>();
            // TODO better way to determine native
            //if path.starts_with("/Script/") {
            let mut names = vec![];
            for item in full_obj.names().iter()? {
                let key = item.a().read()?;
                let value = item.b().read()?;
                names.push((key, value));
            }
            objects.insert(
                path,
                ObjectType::Enum(Enum {
                    object: read_object(&obj.cast())?,
                    cpp_type: full_obj.cpp_type().read()?,
                    names,
                }),
            );
            //}
        } else if path.starts_with("/Script/") {
            let obj = obj.cast::<UObject>();
            objects.insert(path, ObjectType::Object(read_object(&obj)?));
            //println!("{path:?} {:?}", f);
        }
    }
    std::fs::write("../fsd.json", serde_json::to_vec(&objects)?)?;

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_drg() -> Result<()> {
        dump(Input::Process(1762932))
    }
}
