mod containers;

use std::collections::BTreeMap;

use anyhow::{Context as _, Result};
use bitflags::Flags;
use patternsleuth::resolvers::impl_try_collector;
use read_process_memory::{CopyAddress as _, Pid, ProcessHandle};
use serde::Serialize;
use ue_reflection::{
    Class, EClassCastFlags, EClassFlags, EFunctionFlags, EStructFlags, Enum, Function, ObjectType,
    Property, PropertyType, Struct,
};

use crate::containers::{
    ExternalPtr, FArrayProperty, FBoolProperty, FByteProperty, FEnumProperty, FField,
    FInterfaceProperty, FLazyObjectProperty, FMapProperty, FObjectProperty, FProperty,
    FSetProperty, FSoftObjectProperty, FStructProperty, FUObjectArray, FWeakObjectProperty, Mem,
    MemCache, PtrFNamePool, TTuple, UClass, UEnum, UFunction, UObject, UScriptStruct, UStruct,
};

impl_try_collector! {
    #[derive(Debug, PartialEq, Clone)]
    struct DrgResolution {
        guobject_array: patternsleuth::resolvers::unreal::guobject_array::GUObjectArray,
        fname_pool: patternsleuth::resolvers::unreal::fname::FNamePool,
    }
}
impl Mem for ProcessHandle {
    fn read_buf(&self, address: usize, buf: &mut [u8]) -> Result<()> {
        self.copy_address(address, buf)
            .with_context(|| format!("reading {} bytes at 0x{:x}", buf.len(), address))?;
        Ok(())
    }
}

struct Ctx<M: Mem> {
    mem: M,
    fnamepool: PtrFNamePool,
}
impl<M: Mem> Mem for Ctx<M> {
    fn read_buf(&self, address: usize, buf: &mut [u8]) -> Result<()> {
        self.mem.read_buf(address, buf)
    }
}

// TODO
// [ ] UEnum
// [ ] UScriptStruct
// [ ] UStruct?
// [ ] parent clasess
// [ ] interfaces
// [ ] functions
// [ ] functions signatures
// [ ] native function pointers
// [ ] dynamic structs
// [ ] ue version info

fn read_path<M: Mem>(mem: &Ctx<M>, obj: &UObject) -> Result<String> {
    let mut components = vec![];
    let name = mem.fnamepool.read(mem, obj.NamePrivate)?;

    let mut outer = obj.OuterPrivate;
    components.push(name);
    while let Some(o) = outer.read_opt(mem)? {
        components.push(mem.fnamepool.read(mem, o.NamePrivate)?);
        outer = o.OuterPrivate;
    }
    components.reverse();
    Ok(components.join("."))
}

fn map_prop<M: Mem>(mem: &Ctx<M>, ptr: ExternalPtr<FField>) -> Result<Option<Property>> {
    let name = mem.fnamepool.read(mem, ptr.name_private().read(mem)?)?;
    let field_class = ptr.class_private().read(mem)?.read(mem)?;
    let f = field_class.CastFlags;

    if !f.contains(EClassCastFlags::CASTCLASS_FProperty) {
        return Ok(None);
    }

    let t = if f.contains(EClassCastFlags::CASTCLASS_FStructProperty) {
        let prop = ptr.cast::<FStructProperty>().read(mem)?;
        let s = read_path(
            mem,
            &prop.struct_.read_opt(mem)?.unwrap().ustruct.ufield.uobject,
        )?;
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
        let prop = ptr.cast::<FBoolProperty>().read(mem)?;
        PropertyType::Bool {
            field_size: prop.FieldSize,
            byte_offset: prop.ByteOffset,
            byte_mask: prop.ByteMask,
            field_mask: prop.FieldMask,
        }
    } else if f.contains(EClassCastFlags::CASTCLASS_FArrayProperty) {
        let prop = ptr.cast::<FArrayProperty>().read(mem)?;
        PropertyType::Array {
            inner: map_prop(mem, prop.inner.cast())?.unwrap().r#type.into(),
        }
    } else if f.contains(EClassCastFlags::CASTCLASS_FEnumProperty) {
        let prop = ptr.cast::<FEnumProperty>().read(mem)?;
        PropertyType::Enum {
            container: map_prop(mem, prop.underlying_prop.cast())?
                .unwrap()
                .r#type
                .into(),
            r#enum: read_path(mem, &prop.enum_.read(mem)?.ufield.uobject)?,
        }
    } else if f.contains(EClassCastFlags::CASTCLASS_FMapProperty) {
        let prop = ptr.cast::<FMapProperty>().read(mem)?;
        PropertyType::Map {
            key_prop: map_prop(mem, prop.key_prop.cast())?.unwrap().r#type.into(),
            value_prop: map_prop(mem, prop.value_prop.cast())?
                .unwrap()
                .r#type
                .into(),
        }
    } else if f.contains(EClassCastFlags::CASTCLASS_FSetProperty) {
        let prop = ptr.cast::<FSetProperty>().read(mem)?;
        PropertyType::Set {
            key_prop: map_prop(mem, prop.element_prop.cast())?
                .unwrap()
                .r#type
                .into(),
        }
    } else if f.contains(EClassCastFlags::CASTCLASS_FFloatProperty) {
        PropertyType::Float
    } else if f.contains(EClassCastFlags::CASTCLASS_FDoubleProperty) {
        PropertyType::Double
    } else if f.contains(EClassCastFlags::CASTCLASS_FByteProperty) {
        let prop = ptr.cast::<FByteProperty>().read(mem)?;
        PropertyType::Byte {
            r#enum: prop
                .enum_
                .read_opt(mem)?
                .map(|e| read_path(mem, &e.ufield.uobject))
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
        let prop = ptr.cast::<FObjectProperty>().read(mem)?;
        let c = read_path(
            mem,
            &prop
                .property_class
                .read_opt(mem)?
                .unwrap()
                .ustruct
                .ufield
                .uobject,
        )?;
        PropertyType::Object { class: c }
    } else if f.contains(EClassCastFlags::CASTCLASS_FWeakObjectProperty) {
        let prop = ptr.cast::<FWeakObjectProperty>().read(mem)?;
        let c = read_path(
            mem,
            &prop
                .property_class
                .read_opt(mem)?
                .unwrap()
                .ustruct
                .ufield
                .uobject,
        )?;
        PropertyType::WeakObject { class: c }
    } else if f.contains(EClassCastFlags::CASTCLASS_FSoftObjectProperty) {
        let prop = ptr.cast::<FSoftObjectProperty>().read(mem)?;
        let c = read_path(
            mem,
            &prop
                .property_class
                .read_opt(mem)?
                .unwrap()
                .ustruct
                .ufield
                .uobject,
        )?;
        PropertyType::SoftObject { class: c }
    } else if f.contains(EClassCastFlags::CASTCLASS_FLazyObjectProperty) {
        let prop = ptr.cast::<FLazyObjectProperty>().read(mem)?;
        let c = read_path(
            mem,
            &prop
                .property_class
                .read_opt(mem)?
                .unwrap()
                .ustruct
                .ufield
                .uobject,
        )?;
        PropertyType::LazyObject { class: c }
    } else if f.contains(EClassCastFlags::CASTCLASS_FInterfaceProperty) {
        let prop = ptr.cast::<FInterfaceProperty>().read(mem)?;
        let c = read_path(
            mem,
            &prop
                .interface_class
                .read_opt(mem)?
                .unwrap()
                .ustruct
                .ufield
                .uobject,
        )?;
        PropertyType::Interface { class: c }
    } else if f.contains(EClassCastFlags::CASTCLASS_FFieldPathProperty) {
        // TODO
        PropertyType::FieldPath
    } else {
        unimplemented!("{f:?}");
    };

    let prop = ptr.cast::<FProperty>().read(mem)?;
    Ok(Some(Property {
        name,
        offset: prop.Offset_Internal as usize,
        size: prop.ElementSize as usize,
        flags: prop.PropertyFlags,
        r#type: t,
    }))
}

pub fn dump(pid: i32) -> Result<()> {
    let mem: ProcessHandle = (pid as Pid).try_into()?;
    let mem = MemCache::wrap(mem);

    let results = patternsleuth::process::external::read_image_from_pid(pid)?
        .resolve(DrgResolution::resolver())?;

    let guobjectarray = ExternalPtr::<FUObjectArray>::new(results.guobject_array.0);
    let fnamepool = PtrFNamePool(results.fname_pool.0);

    println!("GUObjectArray = {guobjectarray:x?} FNamePool = {fnamepool:x?}");

    let uobject_array = guobjectarray.read(&mem)?;

    let mem = Ctx { mem, fnamepool };

    let mut objects = BTreeMap::<String, ObjectType>::default();

    for i in 0..uobject_array.ObjObjects.NumElements {
        let obj_item = uobject_array.ObjObjects.read_item(&mem, i as usize)?;
        let Some(obj) = obj_item.Object.read_opt(&mem)? else {
            continue;
        };
        let Some(class) = obj.ClassPrivate.read_opt(&mem)? else {
            continue;
        };

        let path = read_path(&mem, &obj)?;

        fn read_struct(mem: &Ctx<impl Mem>, obj: &UStruct) -> Result<Struct> {
            let mut properties = vec![];
            let mut field = obj.ChildProperties;
            while !field.is_null() {
                if let Some(prop) = map_prop(mem, field)? {
                    properties.push(prop);
                }
                field = field.next().read(mem)?;
            }
            let super_struct = obj
                .SuperStruct
                .read_opt(mem)?
                .map(|s| read_path(mem, &s.ufield.uobject))
                .transpose()?;
            Ok(Struct {
                super_struct,
                properties,
            })
        }

        //if (path.starts_with("/Script/")) != !obj.ObjectFlags.contains(EObjectFlags::RF_WasLoaded) {
        //    println!("{path:?} {:?}", obj.ObjectFlags);
        //}

        let f = class.ClassCastFlags;
        if f.contains(EClassCastFlags::CASTCLASS_UClass) {
            let full_obj = obj_item.Object.cast::<UClass>().read(&mem)?;
            if full_obj.ClassFlags.contains(EClassFlags::CLASS_Native) {
                objects.insert(
                    path,
                    ObjectType::Class(Class {
                        r#struct: read_struct(&mem, &full_obj.ustruct)?,
                    }),
                );
            }
        } else if f.contains(EClassCastFlags::CASTCLASS_UFunction) {
            let full_obj = obj_item.Object.cast::<UFunction>().read(&mem)?;
            if full_obj.FunctionFlags.contains(EFunctionFlags::FUNC_Native) {
                objects.insert(
                    path,
                    ObjectType::Function(Function {
                        r#struct: read_struct(&mem, &full_obj.ustruct)?,
                    }),
                );
            }
        } else if f.contains(EClassCastFlags::CASTCLASS_UScriptStruct) {
            let full_obj = obj_item.Object.cast::<UScriptStruct>().read(&mem)?;
            if full_obj.StructFlags.contains(EStructFlags::STRUCT_Native) {
                objects.insert(
                    path,
                    ObjectType::Struct(read_struct(&mem, &full_obj.ustruct)?),
                );
            }
        } else if f.contains(EClassCastFlags::CASTCLASS_UEnum) {
            let full_obj = obj_item.Object.cast::<UEnum>().read(&mem)?;
            // TODO better way to determine native
            if path.starts_with("/Script/") {
                let mut names = vec![];
                for TTuple { key, value } in full_obj.Names.read(&mem)? {
                    names.push((mem.fnamepool.read(&mem, key)?, value));
                }
                objects.insert(
                    path,
                    ObjectType::Enum(Enum {
                        cpp_type: full_obj.CppType.read(&mem)?,
                        names,
                    }),
                );
            }
        } else if path.starts_with("/Script/") {
            println!("{path:?} {:?}", f);
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
        dump(1275510)
    }
}
