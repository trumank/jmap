use std::collections::{BTreeMap, BTreeSet};

use bytemuck::{Pod, Zeroable};
use ordered_float::OrderedFloat;
use ordermap::OrderMap;
use serde::{Deserialize, Serialize};

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, Serialize, Deserialize, Pod, Zeroable)]
    #[repr(C)]
    pub struct EObjectFlags: u32 {
        const _ = !0;

        const RF_NoFlags = 0x0000;
        const RF_Public = 0x0001;
        const RF_Standalone = 0x0002;
        const RF_MarkAsNative = 0x0004;
        const RF_Transactional = 0x0008;
        const RF_ClassDefaultObject = 0x0010;
        const RF_ArchetypeObject = 0x0020;
        const RF_Transient = 0x0040;
        const RF_MarkAsRootSet = 0x0080;
        const RF_TagGarbageTemp = 0x0100;
        const RF_NeedInitialization = 0x0200;
        const RF_NeedLoad = 0x0400;
        const RF_KeepForCooker = 0x0800;
        const RF_NeedPostLoad = 0x1000;
        const RF_NeedPostLoadSubobjects = 0x2000;
        const RF_NewerVersionExists = 0x4000;
        const RF_BeginDestroyed = 0x8000;
        const RF_FinishDestroyed = 0x00010000;
        const RF_BeingRegenerated = 0x00020000;
        const RF_DefaultSubObject = 0x00040000;
        const RF_WasLoaded = 0x00080000;
        const RF_TextExportTransient = 0x00100000;
        const RF_LoadCompleted = 0x00200000;
        const RF_InheritableComponentTemplate = 0x00400000;
        const RF_DuplicateTransient = 0x00800000;
        const RF_StrongRefOnFrame = 0x01000000;
        const RF_NonPIEDuplicateTransient = 0x02000000;
        const RF_Dynamic = 0x04000000;
        const RF_WillBeLoaded = 0x08000000;
    }

    #[derive(Debug, Clone, Copy, Serialize, Deserialize, Pod, Zeroable)]
    #[repr(C)]
    pub struct EFunctionFlags: u32 {
        const _ = !0;

        const FUNC_None = 0x0000;
        const FUNC_Final = 0x0001;
        const FUNC_RequiredAPI = 0x0002;
        const FUNC_BlueprintAuthorityOnly = 0x0004;
        const FUNC_BlueprintCosmetic = 0x0008;
        const FUNC_Net = 0x0040;
        const FUNC_NetReliable = 0x0080;
        const FUNC_NetRequest = 0x0100;
        const FUNC_Exec = 0x0200;
        const FUNC_Native = 0x0400;
        const FUNC_Event = 0x0800;
        const FUNC_NetResponse = 0x1000;
        const FUNC_Static = 0x2000;
        const FUNC_NetMulticast = 0x4000;
        const FUNC_UbergraphFunction = 0x8000;
        const FUNC_MulticastDelegate = 0x00010000;
        const FUNC_Public = 0x00020000;
        const FUNC_Private = 0x00040000;
        const FUNC_Protected = 0x00080000;
        const FUNC_Delegate = 0x00100000;
        const FUNC_NetServer = 0x00200000;
        const FUNC_HasOutParms = 0x00400000;
        const FUNC_HasDefaults = 0x00800000;
        const FUNC_NetClient = 0x01000000;
        const FUNC_DLLImport = 0x02000000;
        const FUNC_BlueprintCallable = 0x04000000;
        const FUNC_BlueprintEvent = 0x08000000;
        const FUNC_BlueprintPure = 0x10000000;
        const FUNC_EditorOnly = 0x20000000;
        const FUNC_Const = 0x40000000;
        const FUNC_NetValidate = 0x80000000;
        const FUNC_AllFlags = 0xffffffff;
    }

    #[derive(Debug, Clone, Copy, Serialize, Deserialize, Pod, Zeroable)]
    #[repr(C)]
    pub struct EClassFlags: i32 {
        const _ = !0;

        const CLASS_None = 0x0000;
        const CLASS_Abstract = 0x0001;
        const CLASS_DefaultConfig = 0x0002;
        const CLASS_Config = 0x0004;
        const CLASS_Transient = 0x0008;
        const CLASS_Parsed = 0x0010;
        const CLASS_MatchedSerializers = 0x0020;
        const CLASS_ProjectUserConfig = 0x0040;
        const CLASS_Native = 0x0080;
        const CLASS_NoExport = 0x0100;
        const CLASS_NotPlaceable = 0x0200;
        const CLASS_PerObjectConfig = 0x0400;
        const CLASS_ReplicationDataIsSetUp = 0x0800;
        const CLASS_EditInlineNew = 0x1000;
        const CLASS_CollapseCategories = 0x2000;
        const CLASS_Interface = 0x4000;
        const CLASS_CustomConstructor = 0x8000;
        const CLASS_Const = 0x00010000;
        const CLASS_LayoutChanging = 0x00020000;
        const CLASS_CompiledFromBlueprint = 0x00040000;
        const CLASS_MinimalAPI = 0x00080000;
        const CLASS_RequiredAPI = 0x00100000;
        const CLASS_DefaultToInstanced = 0x00200000;
        const CLASS_TokenStreamAssembled = 0x00400000;
        const CLASS_HasInstancedReference = 0x00800000;
        const CLASS_Hidden = 0x01000000;
        const CLASS_Deprecated = 0x02000000;
        const CLASS_HideDropDown = 0x04000000;
        const CLASS_GlobalUserConfig = 0x08000000;
        const CLASS_Intrinsic = 0x10000000;
        const CLASS_Constructed = 0x20000000;
        const CLASS_ConfigDoNotCheckDefaults = 0x40000000;
        const CLASS_NewerVersionExists = i32::MIN;
    }


    #[derive(Debug, Clone, Copy, Serialize, Deserialize, Pod, Zeroable)]
    #[repr(C)]
    pub struct EClassCastFlags : u64 {
        const _ = !0;

        const CASTCLASS_None = 0x0000000000000000;
        const CASTCLASS_UField = 0x0000000000000001;
        const CASTCLASS_FInt8Property = 0x0000000000000002;
        const CASTCLASS_UEnum = 0x0000000000000004;
        const CASTCLASS_UStruct = 0x0000000000000008;
        const CASTCLASS_UScriptStruct = 0x0000000000000010;
        const CASTCLASS_UClass = 0x0000000000000020;
        const CASTCLASS_FByteProperty = 0x0000000000000040;
        const CASTCLASS_FIntProperty = 0x0000000000000080;
        const CASTCLASS_FFloatProperty = 0x0000000000000100;
        const CASTCLASS_FUInt64Property = 0x0000000000000200;
        const CASTCLASS_FClassProperty = 0x0000000000000400;
        const CASTCLASS_FUInt32Property = 0x0000000000000800;
        const CASTCLASS_FInterfaceProperty = 0x0000000000001000;
        const CASTCLASS_FNameProperty = 0x0000000000002000;
        const CASTCLASS_FStrProperty = 0x0000000000004000;
        const CASTCLASS_FProperty = 0x0000000000008000;
        const CASTCLASS_FObjectProperty = 0x0000000000010000;
        const CASTCLASS_FBoolProperty = 0x0000000000020000;
        const CASTCLASS_FUInt16Property = 0x0000000000040000;
        const CASTCLASS_UFunction = 0x0000000000080000;
        const CASTCLASS_FStructProperty = 0x0000000000100000;
        const CASTCLASS_FArrayProperty = 0x0000000000200000;
        const CASTCLASS_FInt64Property = 0x0000000000400000;
        const CASTCLASS_FDelegateProperty = 0x0000000000800000;
        const CASTCLASS_FNumericProperty = 0x0000000001000000;
        const CASTCLASS_FMulticastDelegateProperty = 0x0000000002000000;
        const CASTCLASS_FObjectPropertyBase = 0x0000000004000000;
        const CASTCLASS_FWeakObjectProperty = 0x0000000008000000;
        const CASTCLASS_FLazyObjectProperty = 0x0000000010000000;
        const CASTCLASS_FSoftObjectProperty = 0x0000000020000000;
        const CASTCLASS_FTextProperty = 0x0000000040000000;
        const CASTCLASS_FInt16Property = 0x0000000080000000;
        const CASTCLASS_FDoubleProperty = 0x0000000100000000;
        const CASTCLASS_FSoftClassProperty = 0x0000000200000000;
        const CASTCLASS_UPackage = 0x0000000400000000;
        const CASTCLASS_ULevel = 0x0000000800000000;
        const CASTCLASS_AActor = 0x0000001000000000;
        const CASTCLASS_APlayerController = 0x0000002000000000;
        const CASTCLASS_APawn = 0x0000004000000000;
        const CASTCLASS_USceneComponent = 0x0000008000000000;
        const CASTCLASS_UPrimitiveComponent = 0x0000010000000000;
        const CASTCLASS_USkinnedMeshComponent = 0x0000020000000000;
        const CASTCLASS_USkeletalMeshComponent = 0x0000040000000000;
        const CASTCLASS_UBlueprint = 0x0000080000000000;
        const CASTCLASS_UDelegateFunction = 0x0000100000000000;
        const CASTCLASS_UStaticMeshComponent = 0x0000200000000000;
        const CASTCLASS_FMapProperty = 0x0000400000000000;
        const CASTCLASS_FSetProperty = 0x0000800000000000;
        const CASTCLASS_FEnumProperty = 0x0001000000000000;
        const CASTCLASS_USparseDelegateFunction = 0x0002000000000000;
        const CASTCLASS_FMulticastInlineDelegateProperty = 0x0004000000000000;
        const CASTCLASS_FMulticastSparseDelegateProperty = 0x0008000000000000;
        const CASTCLASS_FFieldPathProperty = 0x0010000000000000;
        const CASTCLASS_FLargeWorldCoordinatesRealProperty = 0x0080000000000000;
        const CASTCLASS_FOptionalProperty = 0x0100000000000000;
        const CASTCLASS_FVerseValueProperty = 0x0200000000000000;
        const CASTCLASS_UVerseVMClass = 0x0400000000000000;
    }

    #[derive(Debug, Clone, Copy, Serialize, Deserialize, Pod, Zeroable)]
    #[repr(C)]
    pub struct  EPropertyFlags: u64 {
        const _ = !0;

        const CPF_None = 0x0000;
        const CPF_Edit = 0x0001;
        const CPF_ConstParm = 0x0002;
        const CPF_BlueprintVisible = 0x0004;
        const CPF_ExportObject = 0x0008;
        const CPF_BlueprintReadOnly = 0x0010;
        const CPF_Net = 0x0020;
        const CPF_EditFixedSize = 0x0040;
        const CPF_Parm = 0x0080;
        const CPF_OutParm = 0x0100;
        const CPF_ZeroConstructor = 0x0200;
        const CPF_ReturnParm = 0x0400;
        const CPF_DisableEditOnTemplate = 0x0800;
        const CPF_NonNullable = 0x1000;
        const CPF_Transient = 0x2000;
        const CPF_Config = 0x4000;
        const CPF_RequiredParm = 0x8000;
        const CPF_DisableEditOnInstance = 0x00010000;
        const CPF_EditConst = 0x00020000;
        const CPF_GlobalConfig = 0x00040000;
        const CPF_InstancedReference = 0x00080000;
        const CPF_ExperimentalExternalObjects = 0x00100000;
        const CPF_DuplicateTransient = 0x00200000;
        const CPF_SaveGame = 0x01000000;
        const CPF_NoClear = 0x02000000;
        const CPF_Virtual = 0x04000000;
        const CPF_ReferenceParm = 0x08000000;
        const CPF_BlueprintAssignable = 0x10000000;
        const CPF_Deprecated = 0x20000000;
        const CPF_IsPlainOldData = 0x40000000;
        const CPF_RepSkip = 0x80000000;
        const CPF_RepNotify = 0x100000000;
        const CPF_Interp = 0x200000000;
        const CPF_NonTransactional = 0x400000000;
        const CPF_EditorOnly = 0x800000000;
        const CPF_NoDestructor = 0x1000000000;
        const CPF_AutoWeak = 0x4000000000;
        const CPF_ContainsInstancedReference = 0x8000000000;
        const CPF_AssetRegistrySearchable = 0x10000000000;
        const CPF_SimpleDisplay = 0x20000000000;
        const CPF_AdvancedDisplay = 0x40000000000;
        const CPF_Protected = 0x80000000000;
        const CPF_BlueprintCallable = 0x100000000000;
        const CPF_BlueprintAuthorityOnly = 0x200000000000;
        const CPF_TextExportTransient = 0x400000000000;
        const CPF_NonPIEDuplicateTransient = 0x800000000000;
        const CPF_ExposeOnSpawn = 0x1000000000000;
        const CPF_PersistentInstance = 0x2000000000000;
        const CPF_UObjectWrapper = 0x4000000000000;
        const CPF_HasGetValueTypeHash = 0x8000000000000;
        const CPF_NativeAccessSpecifierPublic = 0x10000000000000;
        const CPF_NativeAccessSpecifierProtected = 0x20000000000000;
        const CPF_NativeAccessSpecifierPrivate = 0x40000000000000;
        const CPF_SkipSerialization = 0x80000000000000;
        const CPF_TObjectPtr = 0x100000000000000;
        const CPF_ExperimentalOverridableLogic = 0x200000000000000;
        const CPF_ExperimentalAlwaysOverriden = 0x400000000000000;
        const CPF_ExperimentalNeverOverriden = 0x800000000000000;
        const CPF_AllowSelfReference = 0x1000000000000000;
    }

    #[derive(Debug, Clone, Copy, Serialize, Deserialize, Pod, Zeroable)]
    #[repr(C)]
    pub struct EInternalObjectFlags: u32 {
        const _ = !0;

        const None = 0x0;
        const ReachableInCluster = 0x800000;
        const ClusterRoot = 0x1000000;
        const Native = 0x2000000;
        const Async = 0x4000000;
        const AsyncLoading = 0x8000000;
        const Unreachable = 0x10000000;
        const PendingKill = 0x20000000;
        const RootSet = 0x40000000;
        const GarbageCollectionKeepFlags = 0xe000000;
        const AllFlags = 0x7f800000;
    }

    #[derive(Debug, Clone, Copy, Serialize, Deserialize, Pod, Zeroable)]
    #[repr(C)]
    pub struct EStructFlags: i32 {
        const _ = !0;

        const STRUCT_NoFlags = 0x0000;
        const STRUCT_Native = 0x0001;
        const STRUCT_IdenticalNative = 0x0002;
        const STRUCT_HasInstancedReference = 0x0004;
        const STRUCT_NoExport = 0x0008;
        const STRUCT_Atomic = 0x0010;
        const STRUCT_Immutable = 0x0020;
        const STRUCT_AddStructReferencedObjects = 0x0040;
        const STRUCT_RequiredAPI = 0x0200;
        const STRUCT_NetSerializeNative = 0x0400;
        const STRUCT_SerializeNative = 0x0800;
        const STRUCT_CopyNative = 0x1000;
        const STRUCT_IsPlainOldData = 0x2000;
        const STRUCT_NoDestructor = 0x4000;
        const STRUCT_ZeroConstructor = 0x8000;
        const STRUCT_ExportTextItemNative = 0x00010000;
        const STRUCT_ImportTextItemNative = 0x00020000;
        const STRUCT_PostSerializeNative = 0x00040000;
        const STRUCT_SerializeFromMismatchedTag = 0x00080000;
        const STRUCT_NetDeltaSerializeNative = 0x00100000;
        const STRUCT_PostScriptConstruct = 0x00200000;
        const STRUCT_NetSharedSerialization = 0x00400000;
        const STRUCT_Trashed = 0x00800000;
        const STRUCT_Inherit = 0x0014;
        const STRUCT_ComputedFlags = 0x007ffc42;
    }

    #[derive(Debug, Clone, Copy, Serialize, Deserialize, Pod, Zeroable)]
    #[repr(C)]
    pub struct EEnumFlags : u8 {
        const Flags = 0x00000001;
        const NewerVersionExists = 0x00000002;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReflectionData {
    pub image_base_address: u64,
    pub objects: BTreeMap<String, ObjectType>,
    pub vtables: BTreeMap<u64, Vec<u64>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Object {
    pub vtable: u64,
    pub object_flags: EObjectFlags,
    pub outer: Option<String>,
    pub class: String,
    pub children: BTreeSet<String>,
    pub property_values: ValuesWrapper,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Package {
    #[serde(flatten)]
    pub object: Object,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Struct {
    #[serde(flatten)]
    pub object: Object,
    pub super_struct: Option<String>,
    pub properties: Vec<Property>,
    pub properties_size: usize,
    pub min_alignment: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptStruct {
    #[serde(flatten)]
    pub r#struct: Struct,
    pub struct_flags: EStructFlags,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Class {
    #[serde(flatten)]
    pub r#struct: Struct,
    pub class_flags: EClassFlags,
    pub class_cast_flags: EClassCastFlags,
    pub class_default_object: Option<String>,
    /// VTable ptr of any instance of this UClass if found
    pub instance_vtable: Option<u64>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Function {
    #[serde(flatten)]
    pub r#struct: Struct,
    pub function_flags: EFunctionFlags,
    pub func: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Enum {
    #[serde(flatten)]
    pub object: Object,
    pub cpp_type: String,
    pub enum_flags: Option<EEnumFlags>,
    pub cpp_form: ECppForm,
    pub names: Vec<(String, i64)>,
}
#[derive(Debug, Clone, Serialize, Deserialize, strum::FromRepr)]
#[repr(u8)]
pub enum ECppForm {
    Regular,
    Namespaced,
    EnumClass,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ObjectType {
    Object(Object),
    Package(Package),
    Enum(Enum),
    ScriptStruct(ScriptStruct),
    Class(Class),
    Function(Function),
}
impl ObjectType {
    pub fn get_object(&self) -> &Object {
        match self {
            ObjectType::Object(obj) => obj,
            ObjectType::Package(obj) => &obj.object,
            ObjectType::Enum(obj) => &obj.object,
            ObjectType::ScriptStruct(obj) => &obj.r#struct.object,
            ObjectType::Class(obj) => &obj.r#struct.object,
            ObjectType::Function(obj) => &obj.r#struct.object,
        }
    }
    pub fn get_struct(&self) -> Option<&Struct> {
        match self {
            ObjectType::Object(_) => None,
            ObjectType::Package(_) => None,
            ObjectType::Enum(_) => None,
            ObjectType::ScriptStruct(obj) => Some(&obj.r#struct),
            ObjectType::Class(obj) => Some(&obj.r#struct),
            ObjectType::Function(obj) => Some(&obj.r#struct),
        }
    }
    pub fn get_enum(&self) -> Option<&Enum> {
        match self {
            ObjectType::Object(_) => None,
            ObjectType::Package(_) => None,
            ObjectType::Enum(obj) => Some(obj),
            ObjectType::ScriptStruct(_) => None,
            ObjectType::Class(_) => None,
            ObjectType::Function(_) => None,
        }
    }
    pub fn get_class(&self) -> Option<&Class> {
        match self {
            ObjectType::Object(_) => None,
            ObjectType::Package(_) => None,
            ObjectType::Enum(_) => None,
            ObjectType::ScriptStruct(_) => None,
            ObjectType::Class(obj) => Some(obj),
            ObjectType::Function(_) => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Property {
    pub name: String,
    pub offset: usize,
    pub array_dim: usize,
    pub size: usize,
    #[serde(flatten)]
    pub r#type: PropertyType,
    pub flags: EPropertyFlags,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum PropertyType {
    #[serde(rename = "StructProperty")]
    Struct { r#struct: String },
    #[serde(rename = "StrProperty")]
    Str,
    #[serde(rename = "NameProperty")]
    Name,
    #[serde(rename = "TextProperty")]
    Text,
    #[serde(rename = "MulticastInlineDelegateProperty")]
    MulticastInlineDelegate { signature_function: Option<String> },
    #[serde(rename = "MulticastSparseDelegateProperty")]
    MulticastSparseDelegate { signature_function: Option<String> },
    #[serde(rename = "MulticastDelegateProperty")]
    MulticastDelegate { signature_function: Option<String> },
    #[serde(rename = "DelegateProperty")]
    Delegate { signature_function: Option<String> },
    #[serde(rename = "BoolProperty")]
    Bool {
        field_size: u8,
        byte_offset: u8,
        byte_mask: u8,
        field_mask: u8,
    },
    #[serde(rename = "ArrayProperty")]
    Array { inner: Box<Property> },
    #[serde(rename = "EnumProperty")]
    Enum {
        container: Box<Property>,
        r#enum: Option<String>,
    },
    #[serde(rename = "MapProperty")]
    Map {
        key_prop: Box<Property>,
        value_prop: Box<Property>,
    },
    #[serde(rename = "SetProperty")]
    Set { key_prop: Box<Property> },
    #[serde(rename = "FloatProperty")]
    Float,
    #[serde(rename = "DoubleProperty")]
    Double,
    #[serde(rename = "ByteProperty")]
    Byte { r#enum: Option<String> },
    #[serde(rename = "UInt16Property")]
    UInt16,
    #[serde(rename = "UInt32Property")]
    UInt32,
    #[serde(rename = "UInt64Property")]
    UInt64,
    #[serde(rename = "Int8Property")]
    Int8,
    #[serde(rename = "Int16Property")]
    Int16,
    #[serde(rename = "IntProperty")]
    Int,
    #[serde(rename = "Int64Property")]
    Int64,
    #[serde(rename = "ObjectProperty")]
    Object { property_class: String },
    #[serde(rename = "ClassProperty")]
    Class {
        property_class: String,
        meta_class: String,
    },
    #[serde(rename = "WeakObjectProperty")]
    WeakObject { property_class: String },
    #[serde(rename = "SoftObjectProperty")]
    SoftObject { property_class: String },
    #[serde(rename = "SoftClassProperty")]
    SoftClass {
        property_class: String,
        meta_class: String,
    },
    #[serde(rename = "LazyObjectProperty")]
    LazyObject { property_class: String },
    #[serde(rename = "InterfaceProperty")]
    Interface { interface_class: String },
    #[serde(rename = "FieldPathProperty")]
    FieldPath,
    #[serde(rename = "OptionalProperty")]
    Optional { inner: Box<Property> },
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(untagged)]
pub enum PropertyValue {
    Struct(OrderMap<String, PropertyValue>),
    Str(String),
    Name(String),
    Text, // TODO
    MulticastInlineDelegate,
    MulticastSparseDelegate,
    Delegate,
    Bool(bool),
    Array(Vec<PropertyValue>),
    Enum(EnumPropertyValue),
    Map(BTreeMap<PropertyValue, PropertyValue>),
    Set(BTreeSet<PropertyValue>),
    Float(OrderedFloat<f32>),
    Double(OrderedFloat<f64>),
    Byte(BytePropertyValue),
    UInt16(u16),
    UInt32(u32),
    UInt64(u64),
    Int8(i8),
    Int16(i16),
    Int(i32),
    Int64(i64),
    Object(Option<String>),
    WeakObject(String),
    SoftObject(String),
    LazyObject(String),
    Interface(String),
    FieldPath, // TODO
    Optional(Option<Box<PropertyValue>>),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(untagged)]
pub enum EnumPropertyValue {
    Value(i64),
    Name(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(untagged)]
pub enum BytePropertyValue {
    Value(u8),
    Name(String),
}

/// Wrapper for PropertyValues which require external context to properly deserialize
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ValuesWrapper {
    Raw(serde_json::value::Value),
    Value(OrderMap<String, PropertyValue>),
}
impl ValuesWrapper {
    /// Returns Some(values) if already parsed, otherwise None
    pub fn values(&self) -> Option<&OrderMap<String, PropertyValue>> {
        match self {
            ValuesWrapper::Raw(_) => None,
            ValuesWrapper::Value(value) => Some(value),
        }
    }
}
impl From<OrderMap<String, PropertyValue>> for ValuesWrapper {
    fn from(value: OrderMap<String, PropertyValue>) -> Self {
        ValuesWrapper::Value(value)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_deser() {
        let ref_data: ReflectionData = serde_json::from_reader(std::io::BufReader::new(
            std::fs::File::open("../rc.json").unwrap(),
        ))
        .unwrap();
        dbg!(&ref_data.objects);
    }
}
