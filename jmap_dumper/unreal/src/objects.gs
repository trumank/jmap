import unreal::core::{UE_VERSION, int8_t, uint8_t, int16_t, uint16_t, int32_t, uint32_t, int64_t, uint64_t};
import unreal::containers::{TArray, TMap};
import unreal::properties::{
    ZField, ZProperty, ZStructProperty,
    FField
};
import unreal::unreal::{
    FName, FString, STUB,
    FRepRecord, FImplementedInterface, FGCReferenceTokenStream,
    FWindowsCriticalSection, FCriticalSection, FWindowsRWLock,
    FNativeFunctionLookup, FTokenStreamOwner
};

// Core object flags and types
type EObjectFlags = int32_t;
type EClassFlags = int32_t;
type EClassCastFlags = uint64_t;
type EFunctionFlags = uint32_t;
type EStructFlags = uint32_t;

/// TEST
struct FStructBaseChain {
    (FStructBaseChain*)* StructBaseChainArray;
    int32_t NumStructBasesInChainMinusOne;
};

/// TEST
struct FClassBaseChain {
    (FClassBaseChain*)* ClassBaseChainArray;
    int32_t NumClassBasesInChainMinusOne;
};

/// TEST
struct FFastIndexingClassTreeRegistrar {
    if (UE_VERSION >= 408) {
        int32_t ClassTreeIndex;
        int32_t ClassTreeNumChildren;
    }
};


struct ICppStructOps {
    uint64_t Placeholder[4];
};

/// TEST
class FOutputDevice {
    uint64_t VTable; // Implicit VTable
    bool bSuppressEventTag;
    bool bAutoEmitLineTerminator;
};

/// TEST
struct FOutParmRec {
    ZProperty* Property;
    uint8_t* PropAddr;
    FOutParmRec* NextOutParm;
};

/// TEST
struct FFrame : FOutputDevice {
    UFunction* Node;
    UObject* Object;
    uint8_t* Code;
    uint8_t* Locals;

    ZProperty* MostRecentProperty;
    uint8_t* MostRecentPropertyAddress;
    if (UE_VERSION >= 501) uint8_t* MostRecentPropertyContainer;

    // FlowStack array - type changes but size stays same (0x30 bytes)
    uint64_t FlowStack[6]; // Placeholder for TArray variants

    FFrame* PreviousFrame;
    FOutParmRec* OutParms;
    ZField* PropertyChainForCompiledIn;
    UFunction* CurrentNativeFunction;
    if (UE_VERSION >= 501) FFrame* PreviousTrackingFrame;
    if (UE_VERSION >= 409) bool bArrayContextFailed;
    if (UE_VERSION >= 501) bool bAbortingExecution;
};

/// Core class in the Unreal Engine object system. All reflection-enabled types are derived from this class
/// TEST
class UObject {
    uint64_t VTable;
    /// Flags assigned to this object. Flags define the logical state of the object
    EObjectFlags ObjectFlags;
    int32_t InternalIndex;
    UClass* ClassPrivate;
    FName NamePrivate;
    UObject* OuterPrivate;
};

/// TEST
class UField : UObject {
    UField* Next;
};

/// TEST
class UStruct : UField, FStructBaseChain if (UE_VERSION >= 422) {
    UStruct* SuperStruct;
    UField* Children;
    if (UE_VERSION >= 425) FField* ChildProperties;
    int32_t PropertiesSize;
    if (UE_VERSION >= 506) {
        uint16_t MinAlignment;
        uint16_t StructStateFlags;
    } else if (UE_VERSION >= 408) int32_t MinAlignment;
    TArray<uint8_t> Script;
    if (UE_VERSION < 408) int32_t MinAlignment;

    ZProperty* PropertyLink;
    ZProperty* RefLink;
    ZProperty* DestructorLink;
    ZProperty* PostConstructLink;
    if (UE_VERSION == 409) ZProperty* RollbackLink;

    if (UE_VERSION >= 425) TArray<UObject*> ScriptAndPropertyObjectReferences;
    else TArray<UObject*> ScriptObjectReferences;

    if (UE_VERSION >= 425) {
        STUB* UnresolvedScriptProperties;
        if (UE_VERSION < 500) STUB* UnversionedSchema;
        else STUB* UnversionedGameSchema;
    }
};

/// TEST
class UFunction : UStruct {
    type FlagsType = if (UE_VERSION < 417) uint32_t else EFunctionFlags;
    FlagsType FunctionFlags;
    if (UE_VERSION < 418) uint16_t RepOffset;
    uint8_t NumParms;
    uint16_t ParmsSize;
    uint16_t ReturnValueOffset;
    uint16_t RPCId;
    uint16_t RPCResponseId;
    ZProperty* FirstPropertyToInit;
    if (UE_VERSION >= 408) UFunction* EventGraphFunction;
    if (UE_VERSION >= 408) int32_t EventGraphCallOffset;
    void* Func;
};

/// TEST
class UScriptStruct : UStruct {
    EStructFlags StructFlags;
    if (UE_VERSION < 408) ICppStructOps* CppStructOps;
    if (UE_VERSION < 414) bool bCppStructOpsFromBaseClass;
    bool bPrepareCppStructOpsCompleted;
    if (UE_VERSION >= 408) ICppStructOps* CppStructOps;
};

type EEnumFlags = if (UE_VERSION >= 505) uint8_t else uint32_t;
type ECppForm = if (UE_VERSION >= 505) uint8_t else uint32_t;

struct UEnumNameTuple {
    type ValueType = if (UE_VERSION < 415) uint8_t else int64_t;

    FName Name;
    if (UE_VERSION >= 409) ValueType Value;
};

/// TEST
class UEnum : UField {
    FString CppType;

    type ValueType = if (UE_VERSION < 415) uint8_t else int64_t;
    TArray<UEnumNameTuple> Names;

    ECppForm CppForm;

    if (UE_VERSION >= 426) EEnumFlags EnumFlags;
    if (UE_VERSION >= 505) FName EnumPackage;
    if (UE_VERSION >= 415) STUB* EnumDisplayNameFn;
    if (UE_VERSION >= 501 && UE_VERSION < 505) FName EnumPackage;
};

struct ICppClassTypeInfo {};
struct FUObjectCppClassStaticFunctions {
    uint64_t Placeholder;
};

/// TEST
class UClass : UStruct,
    FFastIndexingClassTreeRegistrar if (UE_VERSION >= 408 && UE_VERSION < 414),
    FClassBaseChain if (UE_VERSION >= 414 && UE_VERSION < 422) {

    STUB* ClassConstructor;
    if (UE_VERSION >= 408) STUB* ClassVTableHelperCtorCaller;

    if (UE_VERSION >= 501) STUB* CppClassStaticFunctions;
    else STUB* ClassAddReferencedObjects;

    if (UE_VERSION >= 408 && UE_VERSION < 418 || UE_VERSION >= 500) uint32_t ClassUnique;
    if (UE_VERSION >= 500) {
        int32_t FirstOwnedClassRep;
        bool bCooked;
        bool bLayoutChanging;
    } else if (UE_VERSION >= 418) {
        uint32_t ClassUnique : 1;
        uint32_t bCooked : 1;
    }
    EClassFlags ClassFlags;
    EClassCastFlags ClassCastFlags;
     if (UE_VERSION < 408) int32_t ClassUnique;

    UClass* ClassWithin;

    if (UE_VERSION < 500) UObject* ClassGeneratedBy;
    if (UE_VERSION == 421) ZStructProperty* UberGraphFramePointerProperty;
    FName ClassConfigName;
    if (UE_VERSION >= 408 && UE_VERSION < 418) bool bCooked;
    TArray<FRepRecord> ClassReps;
    TArray<UField*> NetFields;
    if (UE_VERSION >= 425 && UE_VERSION < 500) int32_t FirstOwnedClassRep;
    UObject* ClassDefaultObject;
    if (UE_VERSION == 407) bool bCooked;
    if (UE_VERSION >= 416 && UE_VERSION < 418) ICppClassTypeInfo* CppTypeInfo;
    if (UE_VERSION >= 424) {
        STUB* SparseClassData;
        UScriptStruct* SparseClassDataStruct;
    }
    TMap<FName, UFunction*> FuncMap;
    if (UE_VERSION >= 411 && UE_VERSION < 418) {
        TMap<FName, UFunction*> ParentFuncMap;
        TMap<FName, UFunction*> InterfaceFuncMap;
    }
    if (UE_VERSION >= 502) FWindowsRWLock FuncMapLock;
    if (UE_VERSION >= 418 && UE_VERSION < 503) {
        TMap<FName, UFunction*> SuperFuncMap;
        if (UE_VERSION >= 421) FWindowsRWLock SuperFuncMapLock;
    }
    if (UE_VERSION >= 503) {
        TMap<FName, UFunction*> AllFunctionsCache;
        FWindowsRWLock AllFunctionsCacheLock;
    }
    TArray<FImplementedInterface> Interfaces;
    if (UE_VERSION >= 503) STUB* ReferenceSchema; // UE::GC::FSchemaOwner
    else if (UE_VERSION == 502) FTokenStreamOwner ReferenceTokens; // UE::GC::FTokenStreamOwner
    else {
        FGCReferenceTokenStream ReferenceTokenStream;
        if (UE_VERSION >= 415) FCriticalSection ReferenceTokenStreamCritical;
    }
    TArray<FNativeFunctionLookup> NativeFunctionLookupTable;
};
