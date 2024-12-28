use crate::{
    containers::{FName, FString, TArray, TMap, TTuple},
    mem::{CtxPtr, ExternalPtr, Mem},
};
use anyhow::Result;
use ue_reflection::{
    EClassCastFlags, EClassFlags, EFunctionFlags, EObjectFlags, EPropertyFlags, EStructFlags,
};

#[derive(Debug, Clone)]
#[repr(C)]
pub struct UObject {
    pub vtable: ExternalPtr<usize>,
    pub ObjectFlags: EObjectFlags,
    pub InternalIndex: i32,
    pub ClassPrivate: ExternalPtr<UClass>,
    pub NamePrivate: FName,
    pub OuterPrivate: ExternalPtr<UObject>,
}
impl<C: Clone> CtxPtr<UObject, C> {
    pub fn class_private(&self) -> CtxPtr<ExternalPtr<UClass>, C> {
        self.byte_offset(16).cast()
    }
    pub fn name_private(&self) -> CtxPtr<FName, C> {
        self.byte_offset(24).cast()
    }
    pub fn outer_private(&self) -> CtxPtr<ExternalPtr<UObject>, C> {
        self.byte_offset(32).cast()
    }
}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct UField {
    pub uobject: UObject,
    pub next: ExternalPtr<UField>,
}
impl<C: Clone> CtxPtr<UField, C> {
    pub fn uobject(&self) -> CtxPtr<UObject, C> {
        self.cast()
    }
}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct FStructBaseChain {
    pub StructBaseChainArray: ExternalPtr<ExternalPtr<FStructBaseChain>>,
    pub NumStructBasesInChainMinusOne: i32,
}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct UStruct {
    pub ufield: UField,
    pub base_chain: FStructBaseChain,
    pub SuperStruct: ExternalPtr<UStruct>,
    pub Children: ExternalPtr<UField>,
    pub ChildProperties: ExternalPtr<FField>,
    pub PropertiesSize: i32,
    pub MinAlignment: i32,
    pub Script: TArray<u8>,
    pub PropertyLink: ExternalPtr<FProperty>,
    pub RefLink: ExternalPtr<FProperty>,
    pub DestructorLink: ExternalPtr<FProperty>,
    pub PostConstructLink: ExternalPtr<FProperty>,
    pub ScriptAndPropertyObjectReferences: TArray<ExternalPtr<UObject>>,
    pub UnresolvedScriptProperties: ExternalPtr<()>, // *const TArray<TTuple<TFieldPath<FField>,int>,TSizedDefaultAllocator<32> >,
    pub UnversionedSchema: ExternalPtr<()>,          // *const FUnversionedStructSchema
}
impl<C: Clone> CtxPtr<UStruct, C> {
    pub fn ufield(&self) -> CtxPtr<UField, C> {
        self.cast()
    }
    pub fn super_struct(&self) -> CtxPtr<ExternalPtr<UStruct>, C> {
        self.byte_offset(64).cast()
    }
    pub fn child_properties(&self) -> CtxPtr<ExternalPtr<FField>, C> {
        self.byte_offset(80).cast()
    }
}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct UClass {
    pub ustruct: UStruct,
    pub ClassConstructor: ExternalPtr<()>, //extern "system" fn(*const [const] FObjectInitializer),
    pub ClassVTableHelperCtorCaller: ExternalPtr<()>, //extern "system" fn(*const FVTableHelper) -> *const UObject,
    pub ClassAddReferencedObjects: ExternalPtr<()>, //extern "system" fn(*const UObject, *const FReferenceCollector),
    pub ClassUnique_bCooked: u32,                   /* TODO: figure out how to name it */
    pub ClassFlags: EClassFlags,
    pub ClassCastFlags: EClassCastFlags,
    pub ClassWithin: *const UClass,
    pub ClassGeneratedBy: *const UObject,
    pub ClassConfigName: FName,
    pub ClassReps: TArray<()>, //TArray<FRepRecord,TSizedDefaultAllocator<32> >,
    pub NetFields: TArray<ExternalPtr<UField>>,
    pub FirstOwnedClassRep: i32,
    pub ClassDefaultObject: ExternalPtr<UObject>,
    pub SparseClassData: ExternalPtr<()>,
    pub SparseClassDataStruct: ExternalPtr<()>, // *const UScriptStruct
    pub FuncMap: TMap<FName, ExternalPtr<UObject>>, // *const UFunction
    pub SuperFuncMap: TMap<FName, ExternalPtr<UObject>>, //*const UFunction
    pub SuperFuncMapLock: u64,                  //FWindowsRWLock,
    pub Interfaces: TArray<()>, //TArray<FImplementedInterface,TSizedDefaultAllocator<32> >,
    pub ReferenceTokenStream: [u64; 2], // FGCReferenceTokenStream,
    pub ReferenceTokenStreamCritical: [u64; 5], // FWindowsCriticalSection,
    pub NativeFunctionLookupTable: TArray<()>, //TArray<FNativeFunctionLookup,TSizedDefaultAllocator<32> >,
}
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

#[derive(Debug, Clone)]
#[repr(C)]
pub struct UScriptStruct {
    pub ustruct: UStruct,
    pub StructFlags: EStructFlags,
    pub bPrepareCppStructOpsCompleted: bool,
    pub CppStructOps: ExternalPtr<()>, // UScriptStruct::ICppStructOps
}
impl<C: Clone> CtxPtr<UScriptStruct, C> {
    pub fn ustruct(&self) -> CtxPtr<UStruct, C> {
        self.cast()
    }
    pub fn struct_flags(&self) -> CtxPtr<EStructFlags, C> {
        self.byte_offset(176).cast()
    }
}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct UFunction {
    pub ustruct: UStruct,
    pub FunctionFlags: EFunctionFlags,
    pub NumParms: u8,
    pub ParmsSize: u16,
    pub ReturnValueOffset: u16,
    pub RPCId: u16,
    pub RPCResponseId: u16,
    pub FirstPropertyToInit: ExternalPtr<FProperty>,
    pub EventGraphFunction: ExternalPtr<UFunction>,
    pub EventGraphCallOffset: i32,
    pub Func: ExternalPtr<()>, //extern "system" fn(*const UObject, *const FFrame, *const void),
}
impl<C: Clone> CtxPtr<UFunction, C> {
    pub fn ustruct(&self) -> CtxPtr<UStruct, C> {
        self.cast()
    }
    pub fn function_flags(&self) -> CtxPtr<EFunctionFlags, C> {
        self.byte_offset(176).cast()
    }
}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct UEnum {
    pub ufield: UField,
    pub CppType: FString,
    pub Names: TArray<TTuple<FName, i64>>,
    //CppForm: UEnum::ECppForm,
    //EnumFlags: EEnumFlags,
    //EnumDisplayNameFn: extern "system" fn(i32) -> FText,
}
impl<C: Clone> CtxPtr<UEnum, C> {
    pub fn ufield(&self) -> CtxPtr<UField, C> {
        self.cast()
    }
    pub fn names(&self) -> CtxPtr<TArray<TTuple<FName, i64>>, C> {
        self.byte_offset(64).cast()
    }
}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct FField {
    pub vtable: ExternalPtr<()>,
    pub ClassPrivate: ExternalPtr<FFieldClass>,
    pub Owner: FFieldVariant,
    pub Next: ExternalPtr<FField>,
    pub NamePrivate: FName,
    pub FlagsPrivate: EObjectFlags,
}

impl<C: Clone> CtxPtr<FField, C> {
    pub fn class_private(&self) -> CtxPtr<ExternalPtr<FFieldClass>, C> {
        self.byte_offset(8).cast()
    }
    pub fn next(&self) -> CtxPtr<ExternalPtr<FField>, C> {
        self.byte_offset(32).cast()
    }
    pub fn name_private(&self) -> CtxPtr<FName, C> {
        self.byte_offset(40).cast()
    }
}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct FFieldClass {
    pub Name: FName,
    pub Id: u64,
    pub CastFlags: EClassCastFlags,
    pub ClassFlags: EClassFlags,
    pub SuperClass: *const FFieldClass,
    pub DefaultObject: *const FField,
    pub ConstructFn: ExternalPtr<()>, //extern "system" fn(*const [const] FFieldVariant, *const [const] FName, EObjectFlags) -> *const FField,
    pub UnqiueNameIndexCounter: FThreadSafeCounter,
}

#[derive(Clone)]
#[repr(C)]
pub struct FFieldVariant {
    pub Container: FFieldVariant_FFieldObjectUnion,
    pub bIsUObject: bool,
}
impl std::fmt::Debug for FFieldVariant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut fmt = f.debug_struct("FFieldVariant");
        match self.bIsUObject {
            true => fmt.field("object", unsafe { &self.Container.object }),
            false => fmt.field("field", unsafe { &self.Container.field }),
        }
        .finish()
    }
}

#[derive(Clone, Copy)]
#[repr(C)]
pub union FFieldVariant_FFieldObjectUnion {
    pub field: ExternalPtr<FField>,
    pub object: ExternalPtr<UObject>,
}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct FThreadSafeCounter {
    pub counter: i32,
}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct FProperty {
    pub ffield: FField,
    pub ArrayDim: i32,
    pub ElementSize: i32,
    pub PropertyFlags: EPropertyFlags,
    pub RepIndex: u16,
    pub BlueprintReplicationCondition: u8, //TEnumAsByte<enum ELifetimeCondition>,
    pub Offset_Internal: i32,
    pub RepNotifyFunc: FName,
    pub PropertyLinkNext: ExternalPtr<FProperty>,
    pub NextRef: ExternalPtr<FProperty>,
    pub DestructorLinkNext: ExternalPtr<FProperty>,
    pub PostConstructLinkNext: ExternalPtr<FProperty>,
}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct FBoolProperty {
    pub fproperty: FProperty,
    pub FieldSize: u8,
    pub ByteOffset: u8,
    pub ByteMask: u8,
    pub FieldMask: u8,
}
#[derive(Debug, Clone)]
#[repr(C)]
pub struct FObjectProperty {
    pub fproperty: FProperty,
    pub property_class: ExternalPtr<UClass>,
}
impl<C: Clone> CtxPtr<FObjectProperty, C> {
    pub fn property_class(&self) -> CtxPtr<ExternalPtr<UClass>, C> {
        self.byte_offset(120).cast()
    }
}
#[derive(Debug, Clone)]
#[repr(C)]
pub struct FSoftObjectProperty {
    pub fproperty: FProperty,
    pub property_class: ExternalPtr<UClass>,
}
impl<C: Clone> CtxPtr<FSoftObjectProperty, C> {
    pub fn property_class(&self) -> CtxPtr<ExternalPtr<UClass>, C> {
        self.byte_offset(120).cast()
    }
}
#[derive(Debug, Clone)]
#[repr(C)]
pub struct FWeakObjectProperty {
    pub fproperty: FProperty,
    pub property_class: ExternalPtr<UClass>,
}
impl<C: Clone> CtxPtr<FWeakObjectProperty, C> {
    pub fn property_class(&self) -> CtxPtr<ExternalPtr<UClass>, C> {
        self.byte_offset(120).cast()
    }
}
#[derive(Debug, Clone)]
#[repr(C)]
pub struct FLazyObjectProperty {
    pub fproperty: FProperty,
    pub property_class: ExternalPtr<UClass>,
}
impl<C: Clone> CtxPtr<FLazyObjectProperty, C> {
    pub fn property_class(&self) -> CtxPtr<ExternalPtr<UClass>, C> {
        self.byte_offset(120).cast()
    }
}
#[derive(Debug, Clone)]
#[repr(C)]
pub struct FInterfaceProperty {
    pub fproperty: FProperty,
    pub interface_class: ExternalPtr<UClass>,
}
impl<C: Clone> CtxPtr<FInterfaceProperty, C> {
    pub fn interface_class(&self) -> CtxPtr<ExternalPtr<UClass>, C> {
        self.byte_offset(120).cast()
    }
}
#[derive(Debug, Clone)]
#[repr(C)]
pub struct FArrayProperty {
    pub fproperty: FProperty,
    pub inner: ExternalPtr<FProperty>,
    pub array_flags: u32, //EArrayPropertyFlags,
}
impl<C: Clone> CtxPtr<FArrayProperty, C> {
    pub fn inner(&self) -> CtxPtr<ExternalPtr<FProperty>, C> {
        self.byte_offset(120).cast()
    }
}
#[derive(Debug, Clone)]
#[repr(C)]
pub struct FStructProperty {
    pub fproperty: FProperty,
    pub struct_: ExternalPtr<UScriptStruct>,
}
impl<C: Clone> CtxPtr<FStructProperty, C> {
    pub fn struct_(&self) -> CtxPtr<ExternalPtr<UScriptStruct>, C> {
        self.byte_offset(120).cast()
    }
}
#[derive(Debug, Clone)]
#[repr(C)]
pub struct FMapProperty {
    pub fproperty: FProperty,
    pub key_prop: ExternalPtr<FProperty>,
    pub value_prop: ExternalPtr<FProperty>,
    //pub map_layout: FScriptMapLayout,
    //pub map_flags: EMapPropertyFlags,
}
impl<C: Clone> CtxPtr<FMapProperty, C> {
    pub fn key_prop(&self) -> CtxPtr<ExternalPtr<FProperty>, C> {
        self.byte_offset(120).cast()
    }
    pub fn value_prop(&self) -> CtxPtr<ExternalPtr<FProperty>, C> {
        self.byte_offset(128).cast()
    }
}
#[derive(Debug, Clone)]
#[repr(C)]
pub struct FSetProperty {
    pub fproperty: FProperty,
    pub element_prop: ExternalPtr<FProperty>,
    //pub set_layout: FScriptSetLayout,
}
impl<C: Clone> CtxPtr<FSetProperty, C> {
    pub fn element_prop(&self) -> CtxPtr<ExternalPtr<FProperty>, C> {
        self.byte_offset(120).cast()
    }
}
#[derive(Debug, Clone)]
#[repr(C)]
pub struct FEnumProperty {
    pub fproperty: FProperty,
    pub underlying_prop: ExternalPtr<FProperty>, // FNumericProperty
    pub enum_: ExternalPtr<UEnum>,               // FNumericProperty
                                                 //pub set_layout: FScriptSetLayout,
}
impl<C: Clone> CtxPtr<FEnumProperty, C> {
    pub fn underlying_prop(&self) -> CtxPtr<ExternalPtr<FProperty>, C> {
        self.byte_offset(120).cast()
    }
    pub fn enum_(&self) -> CtxPtr<ExternalPtr<UEnum>, C> {
        self.byte_offset(128).cast()
    }
}
#[derive(Debug, Clone)]
#[repr(C)]
pub struct FByteProperty {
    pub fproperty: FProperty,
    pub enum_: ExternalPtr<UEnum>,
}
impl<C: Clone> CtxPtr<FByteProperty, C> {
    pub fn enum_(&self) -> CtxPtr<ExternalPtr<UEnum>, C> {
        self.byte_offset(120).cast()
    }
}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct FUObjectItem {
    pub Object: ExternalPtr<UObject>,
    pub Flags: i32,
    pub ClusterRootIndex: i32,
    pub SerialNumber: i32,
}
impl<C: Clone> CtxPtr<FUObjectItem, C> {
    pub fn object(&self) -> CtxPtr<ExternalPtr<UObject>, C> {
        self.byte_offset(0).cast()
    }
}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct FChunkedFixedUObjectArray {
    pub Objects: ExternalPtr<ExternalPtr<FUObjectItem>>,
    pub PreAllocatedObjects: ExternalPtr<FUObjectItem>,
    pub MaxElements: i32,
    pub NumElements: i32,
    pub MaxChunks: i32,
    pub NumChunks: i32,
}
impl FChunkedFixedUObjectArray {
    pub fn read_item(&self, mem: &impl Mem, item: usize) -> Result<FUObjectItem> {
        let max_per_chunk = 64 * 1024;
        let chunk_index = item / max_per_chunk;

        self.Objects
            .offset(chunk_index)
            .read(mem)?
            .offset(item % max_per_chunk)
            .read(mem)
    }
}
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
            .read_ptr()?
            .offset(chunk_index)
            .read_ptr()?
            .offset(item % max_per_chunk))
    }
}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct FUObjectArray {
    pub ObjFirstGCIndex: i32,
    pub ObjLastNonGCIndex: i32,
    pub MaxObjectsNotConsideredByGC: i32,
    pub OpenForDisregardForGC: bool,
    pub ObjObjects: FChunkedFixedUObjectArray,
    // FWindowsCriticalSection ObjObjectsCritical;
    // TLockFreePointerListUnordered<int,64> ObjAvailableList;
    // TArray<FUObjectArray::FUObjectCreateListener *,TSizedDefaultAllocator<32> > UObjectCreateListeners;
    // TArray<FUObjectArray::FUObjectDeleteListener *,TSizedDefaultAllocator<32> > UObjectDeleteListeners;
    // FWindowsCriticalSection UObjectDeleteListenersCritical;
    // FThreadSafeCounter MasterSerialNumber;
}
impl<C: Clone> CtxPtr<FUObjectArray, C> {
    pub fn obj_object(&self) -> CtxPtr<FChunkedFixedUObjectArray, C> {
        self.byte_offset(16).cast()
    }
}
