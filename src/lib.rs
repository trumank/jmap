mod containers;

#[cfg(test)]
mod test {
    use std::{borrow::Cow, collections::BTreeMap};

    use anyhow::{Context, Result};
    use bitflags::Flags;
    use patternsleuth::{PatternConfig, resolvers::impl_try_collector, scanner::Pattern};
    use read_process_memory::{CopyAddress as _, Pid, ProcessHandle};
    use serde::Serialize;

    use crate::containers::{
        EClassCastFlags, EClassFlags, EObjectFlags, EPropertyFlags, ExternalPtr, FArrayProperty,
        FBoolProperty, FByteProperty, FEnumProperty, FField, FInterfaceProperty,
        FLazyObjectProperty, FMapProperty, FNamePool, FObjectProperty, FProperty, FSetProperty,
        FSoftObjectProperty, FStructProperty, FUObjectArray, FWeakObjectProperty, Mem, MemCache,
        PtrFNamePool, UClass, UObject,
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
            self.copy_address(address, buf)?;
            Ok(())
        }
    }

    #[test]
    fn test_drg() -> Result<()> {
        use bytemuck::Pod;

        let pid = 1490227;
        let mem: ProcessHandle = (pid as Pid).try_into()?;
        let mem = MemCache::wrap(mem);

        let results = patternsleuth::process::external::read_image_from_pid(pid)?
            .resolve(DrgResolution::resolver())?;

        let guobjectarray = ExternalPtr::<FUObjectArray>::new(results.guobject_array.0);
        //let fnamepool = ExternalPtr::<FNamePool>::new(results.fname_pool.0);
        let fnamepool = PtrFNamePool(results.fname_pool.0);

        println!("GUObjectArray = {guobjectarray:x?} FNamePool = {fnamepool:x?}");

        let uobject_array = guobjectarray.read(&mem)?;

        struct Ctx {
            mem: MemCache<ProcessHandle>,
            fnamepool: PtrFNamePool,
        }
        impl Mem for Ctx {
            fn read_buf(&self, address: usize, buf: &mut [u8]) -> Result<()> {
                self.mem.read_buf(address, buf)
            }
        }
        let mem = Ctx { mem, fnamepool };

        fn read_path(mem: &Ctx, obj: &UObject) -> Result<String> {
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

        #[derive(Debug, Serialize)]
        struct Property {
            name: String,
            offset: usize,
            size: usize,
            r#type: PropertyType,
            flags: EPropertyFlags,
        }
        #[derive(Debug, Serialize)]
        enum PropertyType {
            Struct {
                r#struct: String,
            },
            Str,
            Name,
            Text,
            MulticastInlineDelegate,
            MulticastSparseDelegate,
            Delegate,
            Bool {
                field_size: u8,
                byte_offset: u8,
                byte_mask: u8,
                field_mask: u8,
            },
            Array {
                inner: Box<PropertyType>,
            },
            Enum {
                container: Box<PropertyType>,
                r#enum: String,
            },
            Map {
                key_prop: Box<PropertyType>,
                value_prop: Box<PropertyType>,
            },
            Set {
                key_prop: Box<PropertyType>,
            },
            Float,
            Double,
            Byte {
                r#enum: Option<String>,
            },
            UInt16,
            UInt32,
            UInt64,
            Int8,
            Int16,
            Int,
            Int64,
            Object {
                class: String,
            },
            WeakObject {
                class: String,
            },
            SoftObject {
                class: String,
            },
            LazyObject {
                class: String,
            },
            Interface {
                class: String,
            },
            FieldPath,
        }
        #[derive(Debug, Serialize)]
        struct Class {
            properties: Vec<Property>,
        }

        fn map_prop(mem: &Ctx, ptr: ExternalPtr<FField>) -> Result<Option<Property>> {
            let field = ptr.read(mem)?;
            let name = mem.fnamepool.read(mem, field.NamePrivate)?;
            let field_class = field.ClassPrivate.read(mem)?;
            let class_name = mem.fnamepool.read(mem, field_class.Name);
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

        let mut classes = BTreeMap::<String, Class>::default();

        for i in 0..uobject_array.ObjObjects.NumElements {
            let obj = uobject_array.ObjObjects.read_item(&mem, i as usize)?;
            let Some(obj_base) = obj.Object.read_opt(&mem)? else {
                continue;
            };

            let name = fnamepool.read(&mem, obj_base.NamePrivate)?;
            if true {
                if let Some(class) = obj_base.ClassPrivate.read_opt(&mem)? {
                    if class
                        .ClassCastFlags
                        .contains(EClassCastFlags::CASTCLASS_UClass)
                    {
                        let full_obj = obj.Object.cast::<UClass>().read(&mem)?;
                        if full_obj.ClassFlags.contains(EClassFlags::CLASS_Native) {
                            let path = read_path(&mem, &obj_base)?;
                            let class_path = read_path(&mem, &class.ustruct.ufield.uobject);
                            println!("{path:?}");
                            //println!("{path:?} {class_path:?} {obj_base:x?} {class:#?}");

                            let mut class = Class { properties: vec![] };

                            let mut field = full_obj.ustruct.ChildProperties;
                            while let Some(next) = field.read_opt(&mem)? {
                                if let Some(prop) = map_prop(&mem, field)? {
                                    println!("{:?}", prop);
                                    class.properties.push(prop);
                                }
                                field = next.Next;
                            }
                            classes.insert(path, class);
                        }
                    }
                }
            }
        }
        std::fs::write("fsd.json", serde_json::to_vec(&classes)?)?;

        Ok(())
    }
}
