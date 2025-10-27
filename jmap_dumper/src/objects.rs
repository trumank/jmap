use crate::{
    containers::{FName, FString, TArray},
    mem::{Ctx, Ptr, VirtSize},
    read_path,
};
use anyhow::Result;
use jmap::{
    EClassCastFlags, EClassFlags, ECppForm, EEnumFlags, EFunctionFlags, EObjectFlags,
    EPropertyFlags, EStructFlags,
};

macro_rules! inherit {
    ($class:ident : UObject) => {
        impl<C: Clone> Ptr<$class, C> {
            #[allow(unused)]
            pub fn uobject(&self) -> Ptr<UObject, C> {
                self.cast()
            }
        }
        impl<C: Ctx> Ptr<$class, C> {
            #[allow(unused)]
            pub fn path(&self) -> Result<String> {
                self.uobject().path()
            }
        }
        impl<C: Ctx> Ptr<$class, C> {
            #[allow(unused)]
            pub fn class_private(&self) -> Ptr<Ptr<UClass, C>, C> {
                self.uobject().class_private()
            }
        }
    };
    ($class:ident : UField) => {
        inherit!($class : UObject);
        impl<C: Clone> Ptr<$class, C> {
            #[allow(unused)]
            pub fn ufield(&self) -> Ptr<UField, C> {
                self.cast()
            }
        }
    };
    ($class:ident : UStruct) => {
        inherit!($class : UObject);
        impl<C: Clone> Ptr<$class, C> {
            #[allow(unused)]
            pub fn ustruct(&self) -> Ptr<UStruct, C> {
                self.cast()
            }
        }
    };

    ($class:ident : FField) => {
        impl<C: Clone> Ptr<$class, C> {
            #[allow(unused)]
            pub fn ffield(&self) -> Ptr<FField, C> {
                self.cast()
            }
        }
    };
    ($class:ident : ZField) => {
        impl<C: Clone> Ptr<$class, C> {
            #[allow(unused)]
            pub fn zfield(&self) -> Ptr<ZField, C> {
                self.cast()
            }
        }
    };
}

#[derive(Clone, Copy)]
pub struct UObject;
impl<C: Ctx> Ptr<UObject, C> {
    pub fn vtable(&self) -> Ptr<usize, C> {
        self.cast()
    }
    pub fn object_flags(&self) -> Ptr<EObjectFlags, C> {
        let offset = self.ctx().struct_member("UObject", "ObjectFlags");
        self.byte_offset(offset).cast()
    }
    pub fn class_private(&self) -> Ptr<Ptr<UClass, C>, C> {
        let offset = self.ctx().struct_member("UObject", "ClassPrivate");
        self.byte_offset(offset).cast()
    }
    pub fn name_private(&self) -> Ptr<FName, C> {
        let offset = self.ctx().struct_member("UObject", "NamePrivate");
        self.byte_offset(offset).cast()
    }
    pub fn outer_private(&self) -> Ptr<Option<Ptr<UObject, C>>, C> {
        let offset = self.ctx().struct_member("UObject", "OuterPrivate");
        self.byte_offset(offset).cast()
    }
}
impl<C: Ctx> Ptr<UObject, C> {
    pub fn path(&self) -> Result<String> {
        read_path(self)
    }
}

#[derive(Clone, Copy)]
pub struct UField;
inherit!(UField : UObject);
impl<C: Ctx> Ptr<UField, C> {
    pub fn next(&self) -> Ptr<Option<Ptr<UField, C>>, C> {
        let offset = self.ctx().struct_member("UField", "Next");
        self.byte_offset(offset).cast()
    }
}

#[derive(Clone, Copy)]
pub struct UStruct;
inherit!(UStruct : UField);
impl<C: Ctx> Ptr<UStruct, C> {
    pub fn super_struct(&self) -> Ptr<Option<Ptr<UStruct, C>>, C> {
        let offset = self.ctx().struct_member("UStruct", "SuperStruct");
        self.byte_offset(offset).cast()
    }
    pub fn children(&self) -> Ptr<Option<Ptr<UField, C>>, C> {
        let offset = self.ctx().struct_member("UStruct", "Children");
        self.byte_offset(offset).cast()
    }
    pub fn child_properties(&self) -> Ptr<Option<Ptr<ZField, C>>, C> {
        let offset = self.ctx().struct_member("UStruct", "ChildProperties");
        self.byte_offset(offset).cast()
    }
    pub fn properties_size(&self) -> Ptr<i32, C> {
        let offset = self.ctx().struct_member("UStruct", "PropertiesSize");
        self.byte_offset(offset).cast()
    }
    pub fn min_alignment(&self) -> Ptr<i32, C> {
        let offset = self.ctx().struct_member("UStruct", "MinAlignment");
        self.byte_offset(offset).cast()
    }
    pub fn script(&self) -> Ptr<TArray<u8>, C> {
        let offset = self.ctx().struct_member("UStruct", "Script");
        self.byte_offset(offset).cast()
    }
}

impl<C: Ctx> Ptr<UStruct, C> {
    pub fn properties(&self, recurse_parents: bool) -> PropertyIterator<C> {
        PropertyIterator {
            current_struct: Some(self.clone()),
            current_field: None,
            recurse_parents,
        }
    }
    pub fn child_fields(&self) -> Ptr<Option<Ptr<ZField, C>>, C> {
        if self.ctx().ue_version() < (4, 25) {
            self.children().cast()
        } else {
            self.child_properties()
        }
    }
}

#[derive(Clone, Copy)]
pub struct UClass;
inherit!(UClass : UStruct);
impl<C: Ctx> Ptr<UClass, C> {
    pub fn class_flags(&self) -> Ptr<EClassFlags, C> {
        let offset = self.ctx().struct_member("UClass", "ClassFlags");
        self.byte_offset(offset).cast()
    }
    pub fn class_cast_flags(&self) -> Ptr<EClassCastFlags, C> {
        let offset = self.ctx().struct_member("UClass", "ClassCastFlags");
        self.byte_offset(offset).cast()
    }
    pub fn class_default_object(&self) -> Ptr<Option<Ptr<UObject, C>>, C> {
        let offset = self.ctx().struct_member("UClass", "ClassDefaultObject");
        self.byte_offset(offset).cast()
    }
}

#[derive(Clone, Copy)]
pub struct UScriptStruct;
inherit!(UScriptStruct : UStruct);
impl<C: Ctx> Ptr<UScriptStruct, C> {
    pub fn struct_flags(&self) -> Ptr<EStructFlags, C> {
        let offset = self.ctx().struct_member("UScriptStruct", "StructFlags");
        self.byte_offset(offset).cast()
    }
}

#[derive(Clone, Copy)]
pub struct UFunction;
inherit!(UFunction : UStruct);
impl<C: Ctx> Ptr<UFunction, C> {
    pub fn function_flags(&self) -> Ptr<EFunctionFlags, C> {
        let offset = self.ctx().struct_member("UFunction", "FunctionFlags");
        self.byte_offset(offset).cast()
    }
    pub fn func(&self) -> Ptr<usize, C> {
        let offset = self.ctx().struct_member("UFunction", "Func");
        self.byte_offset(offset).cast()
    }
}

#[derive(Clone, Copy)]
pub struct UEnum;
inherit!(UEnum : UField);
impl<C: Ctx> Ptr<UEnum, C> {
    pub fn cpp_type(&self) -> Ptr<FString, C> {
        let offset = self.ctx().struct_member("UEnum", "CppType");
        self.byte_offset(offset).cast()
    }
    /// size of element depends on version so up to caller to figure that out
    pub fn names(&self) -> Ptr<TArray<()>, C> {
        let offset = self.ctx().struct_member("UEnum", "Names");
        self.byte_offset(offset).cast()
    }
    pub fn cpp_form(&self) -> Ptr<ECppForm, C> {
        let offset = self.ctx().struct_member("UEnum", "CppForm");
        self.byte_offset(offset).cast()
    }
    pub fn enum_flags(&self) -> Ptr<EEnumFlags, C> {
        let offset = self.ctx().struct_member("UEnum", "EnumFlags");
        self.byte_offset(offset).cast()
    }
}
#[derive(Clone, Copy)]
pub struct UEnumNameTuple;
impl<C: Ctx> Ptr<UEnumNameTuple, C> {
    pub fn name(&self) -> Ptr<FName, C> {
        let offset = self.ctx().struct_member("UEnumNameTuple", "Name");
        self.byte_offset(offset).cast()
    }
    pub fn value(&self) -> Ptr<(), C> {
        let offset = self.ctx().struct_member("UEnumNameTuple", "Value");
        self.byte_offset(offset).cast()
    }
}
impl<C: Ctx> Ptr<UEnum, C> {
    pub fn read_names(&self) -> Result<Vec<(String, i64)>> {
        let mut names = vec![];
        let len = self.names().len()?;
        if len > 0 {
            let data: Ptr<UEnumNameTuple, _> = self.names().data()?.unwrap().cast();
            let size = self.ctx().get_struct("UEnumNameTuple").size;
            let version = self.ctx().ue_version();
            for i in 0..len {
                let elm = data.byte_offset(i * size as usize);
                let name = elm.name().read()?;
                let value = if version < (4, 9) {
                    i as i64
                } else if version < (4, 15) {
                    elm.value().cast::<u8>().read()? as i64
                } else {
                    elm.value().cast::<i64>().read()?
                };
                names.push((name, value));
            }
        }
        Ok(names)
    }
}

#[derive(Clone, Copy)]
pub struct ZField;
impl<C: Ctx> Ptr<ZField, C> {
    pub fn next(&self) -> Ptr<Option<Ptr<ZField, C>>, C> {
        let offset = self.ctx().struct_member("ZField", "Next");
        self.byte_offset(offset).cast()
    }
    pub fn name_private(&self) -> Ptr<FName, C> {
        let offset = self.ctx().struct_member("ZField", "NamePrivate");
        self.byte_offset(offset).cast()
    }
}
impl<C: Ctx> Ptr<ZField, C> {
    pub fn cast_flags(&self) -> Result<EClassCastFlags> {
        if self.ctx().ue_version() < (4, 25) {
            // UField
            let class = self.cast::<UObject>().class_private().read()?;
            class.class_cast_flags().read()
        } else {
            // FField
            let offset = self.ctx().struct_member("FField", "ClassPrivate");
            let class: Ptr<Ptr<FFieldClass, C>, C> = self.byte_offset(offset).cast();
            class.read()?.cast_flags().read()
        }
    }
}

#[derive(Clone, Copy)]
pub struct FFieldClass;
impl<C: Ctx> Ptr<FFieldClass, C> {
    pub fn cast_flags(&self) -> Ptr<EClassCastFlags, C> {
        let offset = self.ctx().struct_member("FFieldClass", "CastFlags");
        self.byte_offset(offset).cast()
    }
}

#[derive(Clone, Copy)]
pub struct ZProperty;
inherit!(ZProperty : ZField);
impl<C: Ctx> Ptr<ZProperty, C> {
    pub fn array_dim(&self) -> Ptr<i32, C> {
        let offset = self.ctx().struct_member("ZProperty", "ArrayDim");
        self.byte_offset(offset).cast()
    }
    pub fn element_size(&self) -> Ptr<i32, C> {
        let offset = self.ctx().struct_member("ZProperty", "ElementSize");
        self.byte_offset(offset).cast()
    }
    pub fn property_flags(&self) -> Ptr<EPropertyFlags, C> {
        let offset = self.ctx().struct_member("ZProperty", "PropertyFlags");
        self.byte_offset(offset).cast()
    }
    pub fn offset_internal(&self) -> Ptr<i32, C> {
        let offset = self.ctx().struct_member("ZProperty", "Offset_Internal");
        self.byte_offset(offset).cast()
    }
}

#[derive(Clone, Copy)]
pub struct ZBoolProperty;
impl<C: Ctx> Ptr<ZBoolProperty, C> {
    pub fn field_size(&self) -> Ptr<u8, C> {
        let offset = self.ctx().struct_member("ZBoolProperty", "FieldSize");
        self.byte_offset(offset).cast()
    }
    pub fn byte_offset_(&self) -> Ptr<u8, C> {
        let offset = self.ctx().struct_member("ZBoolProperty", "ByteOffset");
        self.byte_offset(offset).cast()
    }
    pub fn byte_mask(&self) -> Ptr<u8, C> {
        let offset = self.ctx().struct_member("ZBoolProperty", "ByteMask");
        self.byte_offset(offset).cast()
    }
    pub fn field_mask(&self) -> Ptr<u8, C> {
        let offset = self.ctx().struct_member("ZBoolProperty", "FieldMask");
        self.byte_offset(offset).cast()
    }
}
#[derive(Clone, Copy)]
pub struct ZObjectProperty;
impl<C: Ctx> Ptr<ZObjectProperty, C> {
    pub fn property_class(&self) -> Ptr<Ptr<UClass, C>, C> {
        let offset = self
            .ctx()
            .struct_member("ZObjectPropertyBase", "PropertyClass");
        self.byte_offset(offset).cast()
    }
}
#[derive(Clone, Copy)]
pub struct ZClassProperty;
impl<C: Ctx> Ptr<ZClassProperty, C> {
    pub fn fobject_property(&self) -> Ptr<ZObjectProperty, C> {
        self.cast()
    }
    pub fn meta_class(&self) -> Ptr<Ptr<UClass, C>, C> {
        let offset = self.ctx().struct_member("ZClassProperty", "MetaClass");
        self.byte_offset(offset).cast()
    }
}
#[derive(Clone, Copy)]
pub struct ZSoftObjectProperty;
impl<C: Ctx> Ptr<ZSoftObjectProperty, C> {
    pub fn property_class(&self) -> Ptr<Ptr<UClass, C>, C> {
        let offset = self
            .ctx()
            .struct_member("ZObjectPropertyBase", "PropertyClass");
        self.byte_offset(offset).cast()
    }
}
#[derive(Clone, Copy)]
pub struct ZSoftClassProperty;
impl<C: Ctx> Ptr<ZSoftClassProperty, C> {
    pub fn fsoft_object_property(&self) -> Ptr<ZSoftObjectProperty, C> {
        self.cast()
    }
    pub fn meta_class(&self) -> Ptr<Ptr<UClass, C>, C> {
        let offset = self.ctx().struct_member("ZSoftClassProperty", "MetaClass");
        self.byte_offset(offset).cast()
    }
}
#[derive(Clone, Copy)]
pub struct ZWeakObjectProperty;
impl<C: Ctx> Ptr<ZWeakObjectProperty, C> {
    pub fn property_class(&self) -> Ptr<Ptr<UClass, C>, C> {
        let offset = self
            .ctx()
            .struct_member("ZObjectPropertyBase", "PropertyClass");
        self.byte_offset(offset).cast()
    }
}
#[derive(Clone, Copy)]
pub struct ZLazyObjectProperty;
impl<C: Ctx> Ptr<ZLazyObjectProperty, C> {
    pub fn property_class(&self) -> Ptr<Ptr<UClass, C>, C> {
        let offset = self
            .ctx()
            .struct_member("ZObjectPropertyBase", "PropertyClass");
        self.byte_offset(offset).cast()
    }
}
#[derive(Clone, Copy)]
pub struct ZInterfaceProperty;
impl<C: Ctx> Ptr<ZInterfaceProperty, C> {
    pub fn interface_class(&self) -> Ptr<Ptr<UClass, C>, C> {
        let offset = self
            .ctx()
            .struct_member("ZInterfaceProperty", "InterfaceClass");
        self.byte_offset(offset).cast()
    }
}
#[derive(Clone, Copy)]
pub struct ZArrayProperty;
impl<C: Ctx> Ptr<ZArrayProperty, C> {
    pub fn inner(&self) -> Ptr<Ptr<ZProperty, C>, C> {
        let offset = self.ctx().struct_member("ZArrayProperty", "Inner");
        self.byte_offset(offset).cast()
    }
}
#[derive(Clone, Copy)]
pub struct ZStructProperty;
impl<C: Ctx> Ptr<ZStructProperty, C> {
    pub fn struct_(&self) -> Ptr<Ptr<UScriptStruct, C>, C> {
        let offset = self.ctx().struct_member("ZStructProperty", "Struct");
        self.byte_offset(offset).cast()
    }
}
#[derive(Clone, Copy)]
pub struct ZMapProperty;
impl<C: Ctx> Ptr<ZMapProperty, C> {
    pub fn key_prop(&self) -> Ptr<Ptr<ZProperty, C>, C> {
        let offset = self.ctx().struct_member("ZMapProperty", "KeyProp");
        self.byte_offset(offset).cast()
    }
    pub fn value_prop(&self) -> Ptr<Ptr<ZProperty, C>, C> {
        let offset = self.ctx().struct_member("ZMapProperty", "ValueProp");
        self.byte_offset(offset).cast()
    }
}
#[derive(Clone, Copy)]
pub struct ZSetProperty;
impl<C: Ctx> Ptr<ZSetProperty, C> {
    pub fn element_prop(&self) -> Ptr<Ptr<ZProperty, C>, C> {
        let offset = self.ctx().struct_member("ZSetProperty", "ElementProp");
        self.byte_offset(offset).cast()
    }
}
#[derive(Clone, Copy)]
pub struct ZEnumProperty;
impl<C: Ctx> Ptr<ZEnumProperty, C> {
    pub fn underlying_prop(&self) -> Ptr<Ptr<ZProperty, C>, C> {
        let offset = self.ctx().struct_member("ZEnumProperty", "UnderlyingProp");
        self.byte_offset(offset).cast()
    }
    pub fn enum_(&self) -> Ptr<Option<Ptr<UEnum, C>>, C> {
        let offset = self.ctx().struct_member("ZEnumProperty", "Enum");
        self.byte_offset(offset).cast()
    }
}
#[derive(Clone, Copy)]
pub struct ZByteProperty;
impl<C: Ctx> Ptr<ZByteProperty, C> {
    pub fn enum_(&self) -> Ptr<Option<Ptr<UEnum, C>>, C> {
        let offset = self.ctx().struct_member("ZByteProperty", "Enum");
        self.byte_offset(offset).cast()
    }
}
#[derive(Clone, Copy)]
pub struct FOptionalProperty;
impl<C: Ctx> Ptr<FOptionalProperty, C> {
    pub fn value_property(&self) -> Ptr<Ptr<ZProperty, C>, C> {
        // TODO implement struct inheritence. for now calculate offset manually
        let parent = self.ctx().get_struct("ZProperty").size as usize;
        let offset = self
            .ctx()
            .struct_member("FOptionalPropertyLayout", "ValueProperty");
        self.byte_offset(parent + offset).cast()
    }
}

#[derive(Clone, Copy)]
pub struct ZDelegateProperty;
impl<C: Ctx> Ptr<ZDelegateProperty, C> {
    pub fn signature_function(&self) -> Ptr<Option<Ptr<UFunction, C>>, C> {
        let offset = self
            .ctx()
            .struct_member("ZDelegateProperty", "SignatureFunction");
        self.byte_offset(offset).cast()
    }
}
#[derive(Clone, Copy)]
pub struct ZMulticastDelegateProperty;
impl<C: Ctx> Ptr<ZMulticastDelegateProperty, C> {
    pub fn signature_function(&self) -> Ptr<Option<Ptr<UFunction, C>>, C> {
        let offset = self
            .ctx()
            .struct_member("ZMulticastDelegateProperty", "SignatureFunction");
        self.byte_offset(offset).cast()
    }
}

#[derive(Clone, Copy)]
pub struct FUObjectItem;
impl<C: Ctx> Ptr<FUObjectItem, C> {
    pub fn object(&self) -> Ptr<Option<Ptr<UObject, C>>, C> {
        self.byte_offset(0).cast()
    }
}
impl<C: Ctx> VirtSize<C> for FUObjectItem {
    fn size(ctx: &C) -> usize {
        ctx.get_struct("FUObjectItem").size as usize
    }
}

#[derive(Clone, Copy)]
pub struct FFixedUObjectArray;
impl<C: Ctx> Ptr<FFixedUObjectArray, C> {
    pub fn objects(&self) -> Ptr<Ptr<FUObjectItem, C>, C> {
        let offset = self.ctx().struct_member("FFixedUObjectArray", "Objects");
        self.byte_offset(offset).cast()
    }
    pub fn num_elements(&self) -> Ptr<i32, C> {
        let offset = self
            .ctx()
            .struct_member("FFixedUObjectArray", "NumElements");
        self.byte_offset(offset).cast()
    }
}
impl<C: Ctx> Ptr<FFixedUObjectArray, C> {
    pub fn read_item_ptr(&self, item: usize) -> Result<Ptr<FUObjectItem, C>> {
        Ok(self.objects().read()?.offset(item))
    }
}

#[derive(Clone, Copy)]
pub struct FChunkedFixedUObjectArray;
impl<C: Ctx> Ptr<FChunkedFixedUObjectArray, C> {
    pub fn objects(&self) -> Ptr<Ptr<Ptr<FUObjectItem, C>, C>, C> {
        let offset = self
            .ctx()
            .struct_member("FChunkedFixedUObjectArray", "Objects");
        self.byte_offset(offset).cast()
    }
    pub fn num_elements(&self) -> Ptr<i32, C> {
        let offset = self
            .ctx()
            .struct_member("FChunkedFixedUObjectArray", "NumElements");
        self.byte_offset(offset).cast()
    }
}
impl<C: Ctx> Ptr<FChunkedFixedUObjectArray, C> {
    pub fn read_item_ptr(&self, item: usize) -> Result<Ptr<FUObjectItem, C>> {
        let max_per_chunk = 64 * 1024;
        let chunk_index = item / max_per_chunk;

        Ok(self
            .objects()
            .read()?
            .offset(chunk_index)
            .read()?
            .offset(item % max_per_chunk))
    }
}
#[derive(Clone, Copy)]
pub struct FUObjectArrayOld;
impl<C: Ctx> Ptr<FUObjectArrayOld, C> {
    pub fn chunks(&self) -> Ptr<Ptr<Option<Ptr<UObject, C>>, C>, C> {
        let offset = self.ctx().struct_member("FUObjectArrayOld", "Chunks");
        self.byte_offset(offset).cast()
    }
    pub fn num_elements(&self) -> Ptr<i32, C> {
        let offset = self.ctx().struct_member("FUObjectArrayOld", "NumElements");
        self.byte_offset(offset).cast()
    }
}
impl<C: Ctx> Ptr<FUObjectArrayOld, C> {
    pub fn read_item_ptr(&self, item: usize) -> Result<Option<Ptr<UObject, C>>> {
        let max_per_chunk = 16 * 1024;
        let chunk_index = item / max_per_chunk;

        self.chunks()
            .offset(chunk_index)
            .read()?
            .offset(item % max_per_chunk)
            .read()
    }
}
#[derive(Clone, Copy)]
pub struct FUObjectArrayOlder;
impl<C: Ctx> Ptr<FUObjectArrayOlder, C> {
    pub fn data(&self) -> Ptr<Ptr<Option<Ptr<UObject, C>>, C>, C> {
        self.cast()
    }
    pub fn num_elements(&self) -> Ptr<i32, C> {
        let offset = self.ctx().struct_member("FUObjectArrayOlder", "ArrayNum");
        self.byte_offset(offset).cast()
    }
}
impl<C: Ctx> Ptr<FUObjectArrayOlder, C> {
    pub fn read_item_ptr(&self, item: usize) -> Result<Option<Ptr<UObject, C>>> {
        self.data().read()?.offset(item).read()
    }
}

#[derive(Clone, Copy)]
pub struct FUObjectArray;
impl<C: Ctx> Ptr<FUObjectArray, C> {
    fn obj_objects(&self) -> Ptr<(), C> {
        let offset = self.ctx().struct_member("FUObjectArray", "ObjObjects");
        self.byte_offset(offset).cast()
    }
}
impl<C: Ctx> Ptr<FUObjectArray, C> {
    pub fn read_item_ptr(&self, item: usize) -> Result<Option<Ptr<UObject, C>>> {
        if self.ctx().ue_version() < (4, 8) {
            self.obj_objects()
                .cast::<FUObjectArrayOlder>()
                .read_item_ptr(item)
        } else if self.ctx().ue_version() < (4, 11) {
            self.obj_objects()
                .cast::<FUObjectArrayOld>()
                .read_item_ptr(item)
        } else if self.ctx().ue_version() < (4, 20) {
            self.obj_objects()
                .cast::<FFixedUObjectArray>()
                .read_item_ptr(item)?
                .object()
                .read()
        } else {
            self.obj_objects()
                .cast::<FChunkedFixedUObjectArray>()
                .read_item_ptr(item)?
                .object()
                .read()
        }
    }
    pub fn num_elements(&self) -> Result<i32> {
        if self.ctx().ue_version() < (4, 8) {
            self.obj_objects()
                .cast::<FUObjectArrayOlder>()
                .num_elements()
        } else if self.ctx().ue_version() < (4, 11) {
            self.obj_objects().cast::<FUObjectArrayOld>().num_elements()
        } else if self.ctx().ue_version() < (4, 20) {
            self.obj_objects()
                .cast::<FFixedUObjectArray>()
                .num_elements()
        } else {
            self.obj_objects()
                .cast::<FChunkedFixedUObjectArray>()
                .num_elements()
        }
        .read()
    }
}

#[derive(Clone)]
pub struct PropertyIterator<C: Ctx> {
    current_struct: Option<Ptr<UStruct, C>>,
    current_field: Option<Ptr<ZField, C>>,
    recurse_parents: bool,
}

impl<C: Ctx> Iterator for PropertyIterator<C> {
    type Item = Result<Ptr<ZProperty, C>>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(current) = self.current_field.take() {
                let is_property = match current.cast_flags() {
                    Ok(flags) if flags.contains(EClassCastFlags::CASTCLASS_FProperty) => true,
                    Ok(_) => false,
                    Err(e) => return Some(Err(e)),
                };

                let next = current.next().read();
                self.current_field = match next {
                    Ok(next) => next,
                    Err(e) => return Some(Err(e)),
                };

                if is_property {
                    return Some(Ok(current.cast::<ZProperty>()));
                }
            } else if let Some(current) = self.current_struct.take() {
                self.current_field = match current.child_fields().read() {
                    Ok(children) => children,
                    Err(e) => return Some(Err(e)),
                };

                if self.recurse_parents {
                    self.current_struct = match current.super_struct().read() {
                        Ok(super_struct) => super_struct,
                        Err(e) => return Some(Err(e)),
                    };
                }
            } else {
                return None;
            }
        }
    }
}
