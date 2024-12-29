use crate::{
    containers::{FName, FString, TArray, TTuple},
    mem::{CtxPtr, ExternalPtr, Mem},
};
use anyhow::Result;
use ue_reflection::{EClassCastFlags, EClassFlags, EFunctionFlags, EPropertyFlags, EStructFlags};

#[derive(Clone, Copy)]
pub struct UObject;
impl<C: Clone> CtxPtr<UObject, C> {
    pub fn class_private(&self) -> CtxPtr<ExternalPtr<UClass>, C> {
        self.byte_offset(16).cast()
    }
    pub fn name_private(&self) -> CtxPtr<FName, C> {
        self.byte_offset(24).cast()
    }
    pub fn outer_private(&self) -> CtxPtr<Option<ExternalPtr<UObject>>, C> {
        self.byte_offset(32).cast()
    }
}

#[derive(Clone, Copy)]
pub struct UField;
impl<C: Clone> CtxPtr<UField, C> {
    pub fn uobject(&self) -> CtxPtr<UObject, C> {
        self.cast()
    }
}

#[derive(Clone, Copy)]
pub struct UStruct;
impl<C: Clone> CtxPtr<UStruct, C> {
    pub fn ufield(&self) -> CtxPtr<UField, C> {
        self.cast()
    }
    pub fn super_struct(&self) -> CtxPtr<Option<ExternalPtr<UStruct>>, C> {
        self.byte_offset(64).cast()
    }
    pub fn child_properties(&self) -> CtxPtr<Option<ExternalPtr<FField>>, C> {
        self.byte_offset(80).cast()
    }
}

#[derive(Clone, Copy)]
pub struct UClass;
impl<C: Clone> CtxPtr<UClass, C> {
    pub fn ustruct(&self) -> CtxPtr<UStruct, C> {
        self.cast()
    }
    pub fn class_flags(&self) -> CtxPtr<EClassFlags, C> {
        self.byte_offset(204).cast()
    }
    pub fn class_cast_flags(&self) -> CtxPtr<EClassCastFlags, C> {
        self.byte_offset(208).cast()
    }
}

#[derive(Clone, Copy)]
pub struct UScriptStruct;
impl<C: Clone> CtxPtr<UScriptStruct, C> {
    pub fn ustruct(&self) -> CtxPtr<UStruct, C> {
        self.cast()
    }
    pub fn struct_flags(&self) -> CtxPtr<EStructFlags, C> {
        self.byte_offset(176).cast()
    }
}

#[derive(Clone, Copy)]
pub struct UFunction;
impl<C: Clone> CtxPtr<UFunction, C> {
    pub fn ustruct(&self) -> CtxPtr<UStruct, C> {
        self.cast()
    }
    pub fn function_flags(&self) -> CtxPtr<EFunctionFlags, C> {
        self.byte_offset(176).cast()
    }
}

#[derive(Clone, Copy)]
pub struct UEnum;
impl<C: Clone> CtxPtr<UEnum, C> {
    pub fn ufield(&self) -> CtxPtr<UField, C> {
        self.cast()
    }
    pub fn cpp_type(&self) -> CtxPtr<FString, C> {
        self.byte_offset(48).cast()
    }
    pub fn names(&self) -> CtxPtr<TArray<TTuple<FName, i64>>, C> {
        self.byte_offset(64).cast()
    }
}

#[derive(Clone, Copy)]
pub struct FField;
impl<C: Clone> CtxPtr<FField, C> {
    pub fn class_private(&self) -> CtxPtr<ExternalPtr<FFieldClass>, C> {
        self.byte_offset(8).cast()
    }
    pub fn next(&self) -> CtxPtr<Option<ExternalPtr<FField>>, C> {
        self.byte_offset(32).cast()
    }
    pub fn name_private(&self) -> CtxPtr<FName, C> {
        self.byte_offset(40).cast()
    }
}

#[derive(Clone, Copy)]
pub struct FFieldClass;
impl<C: Clone> CtxPtr<FFieldClass, C> {
    pub fn cast_flags(&self) -> CtxPtr<EClassCastFlags, C> {
        self.byte_offset(16).cast()
    }
}

#[derive(Clone, Copy)]
pub struct FProperty;
impl<C: Clone> CtxPtr<FProperty, C> {
    pub fn ffield(&self) -> CtxPtr<FField, C> {
        self.cast()
    }
    pub fn element_size(&self) -> CtxPtr<i32, C> {
        self.byte_offset(60).cast()
    }
    pub fn property_flags(&self) -> CtxPtr<EPropertyFlags, C> {
        self.byte_offset(64).cast()
    }
    pub fn offset_internal(&self) -> CtxPtr<i32, C> {
        self.byte_offset(76).cast()
    }
}

#[derive(Clone, Copy)]
pub struct FBoolProperty;
impl<C: Clone> CtxPtr<FBoolProperty, C> {
    pub fn field_size(&self) -> CtxPtr<u8, C> {
        self.byte_offset(120).cast()
    }
    pub fn byte_offset_(&self) -> CtxPtr<u8, C> {
        self.byte_offset(121).cast()
    }
    pub fn byte_mask(&self) -> CtxPtr<u8, C> {
        self.byte_offset(122).cast()
    }
    pub fn field_mask(&self) -> CtxPtr<u8, C> {
        self.byte_offset(123).cast()
    }
}
#[derive(Clone, Copy)]
pub struct FObjectProperty;
impl<C: Clone> CtxPtr<FObjectProperty, C> {
    pub fn property_class(&self) -> CtxPtr<ExternalPtr<UClass>, C> {
        self.byte_offset(120).cast()
    }
}
#[derive(Clone, Copy)]
pub struct FSoftObjectProperty;
impl<C: Clone> CtxPtr<FSoftObjectProperty, C> {
    pub fn property_class(&self) -> CtxPtr<ExternalPtr<UClass>, C> {
        self.byte_offset(120).cast()
    }
}
#[derive(Clone, Copy)]
pub struct FWeakObjectProperty;
impl<C: Clone> CtxPtr<FWeakObjectProperty, C> {
    pub fn property_class(&self) -> CtxPtr<ExternalPtr<UClass>, C> {
        self.byte_offset(120).cast()
    }
}
#[derive(Clone, Copy)]
pub struct FLazyObjectProperty;
impl<C: Clone> CtxPtr<FLazyObjectProperty, C> {
    pub fn property_class(&self) -> CtxPtr<ExternalPtr<UClass>, C> {
        self.byte_offset(120).cast()
    }
}
#[derive(Clone, Copy)]
pub struct FInterfaceProperty;
impl<C: Clone> CtxPtr<FInterfaceProperty, C> {
    pub fn interface_class(&self) -> CtxPtr<ExternalPtr<UClass>, C> {
        self.byte_offset(120).cast()
    }
}
#[derive(Clone, Copy)]
pub struct FArrayProperty;
impl<C: Clone> CtxPtr<FArrayProperty, C> {
    pub fn inner(&self) -> CtxPtr<ExternalPtr<FProperty>, C> {
        self.byte_offset(120).cast()
    }
}
#[derive(Clone, Copy)]
pub struct FStructProperty;
impl<C: Clone> CtxPtr<FStructProperty, C> {
    pub fn struct_(&self) -> CtxPtr<ExternalPtr<UScriptStruct>, C> {
        self.byte_offset(120).cast()
    }
}
#[derive(Clone, Copy)]
pub struct FMapProperty;
impl<C: Clone> CtxPtr<FMapProperty, C> {
    pub fn key_prop(&self) -> CtxPtr<ExternalPtr<FProperty>, C> {
        self.byte_offset(120).cast()
    }
    pub fn value_prop(&self) -> CtxPtr<ExternalPtr<FProperty>, C> {
        self.byte_offset(128).cast()
    }
}
#[derive(Clone, Copy)]
pub struct FSetProperty;
impl<C: Clone> CtxPtr<FSetProperty, C> {
    pub fn element_prop(&self) -> CtxPtr<ExternalPtr<FProperty>, C> {
        self.byte_offset(120).cast()
    }
}
#[derive(Clone, Copy)]
pub struct FEnumProperty;
impl<C: Clone> CtxPtr<FEnumProperty, C> {
    pub fn underlying_prop(&self) -> CtxPtr<ExternalPtr<FProperty>, C> {
        self.byte_offset(120).cast()
    }
    pub fn enum_(&self) -> CtxPtr<ExternalPtr<UEnum>, C> {
        self.byte_offset(128).cast()
    }
}
#[derive(Clone, Copy)]
pub struct FByteProperty;
impl<C: Clone> CtxPtr<FByteProperty, C> {
    pub fn enum_(&self) -> CtxPtr<Option<ExternalPtr<UEnum>>, C> {
        self.byte_offset(120).cast()
    }
}

#[derive(Clone, Copy)]
pub struct FUObjectItem;
impl<C: Clone> CtxPtr<FUObjectItem, C> {
    pub fn object(&self) -> CtxPtr<Option<ExternalPtr<UObject>>, C> {
        self.byte_offset(0).cast()
    }
}

#[derive(Clone, Copy)]
pub struct FChunkedFixedUObjectArray;
impl<C: Clone> CtxPtr<FChunkedFixedUObjectArray, C> {
    pub fn objects(&self) -> CtxPtr<ExternalPtr<ExternalPtr<FUObjectItem>>, C> {
        self.byte_offset(0).cast()
    }
    pub fn num_elements(&self) -> CtxPtr<i32, C> {
        self.byte_offset(20).cast()
    }
}
impl<C: Mem + Clone> CtxPtr<FChunkedFixedUObjectArray, C> {
    pub fn read_item_ptr(&self, item: usize) -> Result<CtxPtr<FUObjectItem, C>> {
        let max_per_chunk = 64 * 1024;
        let chunk_index = item / max_per_chunk;

        Ok(self
            .objects()
            .read()?
            .offset(chunk_index)
            .read()?
            .byte_offset(24 * (item % max_per_chunk))) // TODO dynamic struct size
    }
}

#[derive(Clone, Copy)]
pub struct FUObjectArray;
impl<C: Clone> CtxPtr<FUObjectArray, C> {
    pub fn obj_object(&self) -> CtxPtr<FChunkedFixedUObjectArray, C> {
        self.byte_offset(16).cast()
    }
}
