// Unreal Engine Property System

import unreal::core::{UE_VERSION, int32_t, uint32_t, uint64_t, uint16_t, uint8_t};
import unreal::unreal::{FName};
import unreal::uobjectarray::{FThreadSafeCounter};
import unreal::objects::{UField, UClass, UEnum, UScriptStruct, UFunction, EClassFlags};

// Property-related enums and flags
type EPropertyFlags = uint64_t;
type ELifetimeCondition = uint8_t;
type EArrayPropertyFlags = uint8_t;
type EMapPropertyFlags = uint8_t;

// Helper templates
template<typename T>
struct TEnumAsByte {
    T Value;
};

template<typename T>
struct TObjectPtr {
    T* Object; // Memory-identical to raw pointer
};

/// TEST
class FFieldClass {
    FName Name;
    if (UE_VERSION >= 507) EClassFlags ClassFlags;
    uint64_t Id;
    uint64_t CastFlags;
    if (UE_VERSION < 507) EClassFlags ClassFlags;
    FFieldClass* SuperClass;
    FField* DefaultObject;
    void* ConstructFn; // FField*(*)(const FFieldVariant*, const FName*, EObjectFlags)
    if (UE_VERSION < 507) FThreadSafeCounter UnqiueNameIndexCounter;
    else int32_t UniqueNameIndexCounter; // std::atomic<int>
};

/// TEST
struct FFieldVariant {
    uint64_t Container; // FFieldObjectUnion
    if (UE_VERSION < 503) bool bIsUObject;
};

/// TEST
class FField {
    uint64_t VTable; // Implicit VTable for alignment
    FFieldClass* ClassPrivate;
    FFieldVariant Owner;
    FField* Next;
    FName NamePrivate;
    uint32_t FlagsPrivate; // EObjectFlags
};

// Unified type aliases - default to version-appropriate types
type ZFieldBase = if (UE_VERSION >= 425) FField else UField;
struct ZField : ZFieldBase {};

/// TEST
class ZProperty : ZField {
    int32_t ArrayDim;
    int32_t ElementSize;

    if (UE_VERSION < 420) uint64_t PropertyFlags;
    else EPropertyFlags PropertyFlags;

    uint16_t RepIndex;

    if (UE_VERSION < 418) FName RepNotifyFunc;
    if (UE_VERSION >= 418) TEnumAsByte<ELifetimeCondition> BlueprintReplicationCondition;
    int32_t Offset_Internal;
    if (UE_VERSION >= 414 && UE_VERSION < 418) uint32_t BlueprintReplicationCondition;
    if (UE_VERSION >= 418 && UE_VERSION < 503) FName RepNotifyFunc;

    ZProperty* PropertyLinkNext;
    ZProperty* NextRef;
    ZProperty* DestructorLinkNext;
    ZProperty* PostConstructLinkNext;
    if (UE_VERSION == 409) ZProperty* RollbackLinkNext;
    if (UE_VERSION >= 503) FName RepNotifyFunc;
};

// Script layout types
/// TEST
struct FScriptSparseArrayLayout {
    if (UE_VERSION < 422) int32_t ElementOffset;
    int32_t Alignment;
    int32_t Size;
};

/// TEST
struct FScriptSetLayout {
    if (UE_VERSION < 422) int32_t ElementOffset;
    int32_t HashNextIdOffset;
    int32_t HashIndexOffset;
    int32_t Size;
    FScriptSparseArrayLayout SparseArrayLayout;
};

/// TEST
struct FScriptMapLayout {
    if (UE_VERSION < 422) int32_t KeyOffset;
    int32_t ValueOffset;
    FScriptSetLayout SetLayout;
};

/// TEST
struct FOptionalPropertyLayout {
    ZProperty* ValueProperty;
};

// Specific property types
class ZObjectPropertyBase : ZProperty {
    UClass* PropertyClass;
};

class ZObjectProperty : ZObjectPropertyBase {};

class ZClassProperty : ZObjectProperty {
    UClass* MetaClass;
};

class ZNumericProperty : ZProperty {};

class ZEnumProperty : ZProperty {
    ZNumericProperty* UnderlyingProp;
    UEnum* Enum;
};

class ZByteProperty : ZProperty {
    UEnum* Enum;
};

class ZBoolProperty : ZProperty {
    uint8_t FieldSize;
    uint8_t ByteOffset;
    uint8_t ByteMask;
    uint8_t FieldMask;
};

class ZArrayProperty : ZProperty {
    if (UE_VERSION < 503) {
        ZProperty* Inner;
        EArrayPropertyFlags ArrayFlags;
    } else {
        EArrayPropertyFlags ArrayFlags;
        ZProperty* Inner;
    }
};

class ZSetProperty : ZProperty {
    ZProperty* ElementProp;
    FScriptSetLayout SetLayout;
};

class ZMapProperty : ZProperty {
    ZProperty* KeyProp;
    ZProperty* ValueProp;
    FScriptMapLayout MapLayout;
    EMapPropertyFlags MapFlags;
};

class ZInterfaceProperty : ZProperty {
    UClass* InterfaceClass;
};

class ZSoftObjectProperty : ZObjectPropertyBase {};

class ZSoftClassProperty : ZSoftObjectProperty {
    UClass* MetaClass;
};

class ZWeakObjectProperty : ZProperty {};

class ZLazyObjectProperty : ZProperty {};

class ZStructProperty : ZProperty {
    UScriptStruct* Struct;
};

class ZDelegateProperty : ZProperty {
    UFunction* SignatureFunction;
};

class ZMulticastDelegateProperty : ZProperty {
    UFunction* SignatureFunction;
};

class ZMulticastSparseDelegateProperty : ZMulticastDelegateProperty {};

class ZOptionalProperty : ZProperty, FOptionalPropertyLayout {};
