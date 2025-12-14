// Unreal Engine Property System

import unreal::core::{UE_VERSION, int32_t, uint32_t, uint64_t, uint16_t, uint8_t};
import unreal::unreal::{FName};
import unreal::uobjectarray::{FThreadSafeCounter};
import unreal::objects::{UField, UClass, UEnum, UScriptStruct, UFunction, EClassFlags};

// Property-related enums and flags
public type EPropertyFlags = uint64_t;
public type ELifetimeCondition = uint8_t;
public type EArrayPropertyFlags = uint8_t;
public type EMapPropertyFlags = uint8_t;

// Helper templates
template<typename T>
public struct TEnumAsByte {
    T Value;
};

template<typename T>
public struct TObjectPtr {
    T* Object; // Memory-identical to raw pointer
};

/// TEST
public class FFieldClass {
    FName Name;
    uint64_t Id;
    uint64_t CastFlags;
    EClassFlags ClassFlags;
    FFieldClass* SuperClass;
    FField* DefaultObject;
    void* ConstructFn; // FField*(*)(const FFieldVariant*, const FName*, EObjectFlags)
    FThreadSafeCounter UnqiueNameIndexCounter;
};

/// TEST
public struct FFieldVariant {
    uint64_t Container; // FFieldObjectUnion
    if (UE_VERSION < 503) bool bIsUObject;
};

/// TEST
public class FField {
    uint64_t VTable; // Implicit VTable for alignment
    FFieldClass* ClassPrivate;
    FFieldVariant Owner;
    FField* Next;
    FName NamePrivate;
    uint32_t FlagsPrivate; // EObjectFlags
};

// Unified type aliases - default to version-appropriate types
public type ZFieldBase = if (UE_VERSION >= 425) FField else UField;
public struct ZField : ZFieldBase {};

/// TEST
public class ZProperty : ZField {
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
public struct FScriptSparseArrayLayout {
    if (UE_VERSION < 422) int32_t ElementOffset;
    int32_t Alignment;
    int32_t Size;
};

/// TEST
public struct FScriptSetLayout {
    if (UE_VERSION < 422) int32_t ElementOffset;
    int32_t HashNextIdOffset;
    int32_t HashIndexOffset;
    int32_t Size;
    FScriptSparseArrayLayout SparseArrayLayout;
};

/// TEST
public struct FScriptMapLayout {
    if (UE_VERSION < 422) int32_t KeyOffset;
    int32_t ValueOffset;
    FScriptSetLayout SetLayout;
};

/// TEST
public struct FOptionalPropertyLayout {
    ZProperty* ValueProperty;
};

// Specific property types
public class ZObjectPropertyBase : ZProperty {
    UClass* PropertyClass;
};

public class ZObjectProperty : ZObjectPropertyBase {};

public class ZClassProperty : ZObjectProperty {
    UClass* MetaClass;
};

public class ZNumericProperty : ZProperty {};

public class ZEnumProperty : ZProperty {
    ZNumericProperty* UnderlyingProp;
    UEnum* Enum;
};

public class ZByteProperty : ZProperty {
    UEnum* Enum;
};

public class ZBoolProperty : ZProperty {
    uint8_t FieldSize;
    uint8_t ByteOffset;
    uint8_t ByteMask;
    uint8_t FieldMask;
};

public class ZArrayProperty : ZProperty {
    if (UE_VERSION < 503) {
        ZProperty* Inner;
        EArrayPropertyFlags ArrayFlags;
    } else {
        EArrayPropertyFlags ArrayFlags;
        ZProperty* Inner;
    }
};

public class ZSetProperty : ZProperty {
    ZProperty* ElementProp;
    FScriptSetLayout SetLayout;
};

public class ZMapProperty : ZProperty {
    ZProperty* KeyProp;
    ZProperty* ValueProp;
    FScriptMapLayout MapLayout;
    EMapPropertyFlags MapFlags;
};

public class ZInterfaceProperty : ZProperty {
    UClass* InterfaceClass;
};

public class ZSoftObjectProperty : ZObjectPropertyBase {};

public class ZSoftClassProperty : ZSoftObjectProperty {
    UClass* MetaClass;
};

public class ZWeakObjectProperty : ZProperty {};

public class ZLazyObjectProperty : ZProperty {};

public class ZStructProperty : ZProperty {
    UScriptStruct* Struct;
};

public class ZDelegateProperty : ZProperty {
    UFunction* SignatureFunction;
};

public class ZMulticastDelegateProperty : ZProperty {
    UFunction* SignatureFunction;
};

public class ZMulticastSparseDelegateProperty : ZMulticastDelegateProperty {};

public class ZOptionalProperty : ZProperty, FOptionalPropertyLayout {};
