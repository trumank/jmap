input int UE_VERSION;
input int WITH_CASE_PRESERVING_NAME = 0;

// Definitions for types with explicit sizes
type int8_t = char;
type uint8_t = unsigned char;
type int16_t = short int;
type uint16_t = unsigned short int;
type int32_t = int;
type uint32_t = unsigned int;
type int64_t = long long int;
type uint64_t = unsigned long long int;

type intptr_t = if (__address_size == 8) int64_t else int32_t;
type uintptr_t = if (__address_size == 8) uint64_t else uint32_t;

/* Test block comment */
template<typename T>
struct TVector {
    T X;
    T Y;
    T Z;
};

/*
 * Test multi-line block comment
 */
type FVector = TVector<double>;

public struct FScriptElement {};

template<typename InIndexType>
public struct TSizedHeapAllocator {
    type IndexType = InIndexType;
    FScriptElement* Data;
};

template<typename InElementType, typename InAllocator = TSizedHeapAllocator<int32_t>>
public struct TArray {
    type ElementType = InElementType;
    type IndexType = InAllocator::typename IndexType;

    InAllocator AllocatorInstance;
    InAllocator::typename IndexType ArrayNum;
    InAllocator::typename IndexType ArrayMax;
};

struct FNameEntryId {
    uint32_t Value;
};
// FName has different alignment requirements across versions due to union in early versions
struct alignas(if (UE_VERSION < 422) 8 else 4) FName {
    if (UE_VERSION < 422) {
        // Early versions (4.12-4.21): Contains union with uint64_t (8-byte aligned)
        // Union not fully defined due to Gospel limitations, but forces 8-byte alignment
        int32_t ComparisonIndex;
        uint32_t Number;
    } else if (UE_VERSION < 423) {
        // UE 4.22: Transition version, no union, int32_t ComparisonIndex
        int32_t ComparisonIndex;
        uint32_t Number;
    } else {
        // UE 4.23+: Modern version with FNameEntryId
        FNameEntryId ComparisonIndex;
        if (WITH_CASE_PRESERVING_NAME == 1) FNameEntryId DisplayIndex;
        uint32_t Number;
    }
};

struct FStructBaseChain {
    (FStructBaseChain*)* StructBaseChainArray;
    int32_t NumStructBasesInChainMinusOne;
};
struct FClassBaseChain {
    (FClassBaseChain*)* ClassBaseChainArray;
    int32_t NumClassBasesInChainMinusOne;
};

struct FFastIndexingClassTreeRegistrar {
    uint64_t Placeholder;
};

type EPropertyFlags = uint64_t;
type ELifetimeCondition = uint8_t;
type EArrayPropertyFlags = uint8_t;
type EMapPropertyFlags = uint8_t;

template<typename T>
struct TEnumAsByte {
    T Value;
};

template<typename T>
struct TObjectPtr {
    T* Object; // Memory-identical to raw pointer
};

class FFieldClass {
    FName Name;
    uint64_t Id;
    uint64_t CastFlags;
    EClassFlags ClassFlags;
    FFieldClass* SuperClass;
    FField* DefaultObject;
    intptr_t ConstructFn; // FField*(*)(const FFieldVariant*, const FName*, EObjectFlags)
    FThreadSafeCounter UnqiueNameIndexCounter;
};

struct FFieldVariant {
    uint64_t Container; // FFieldObjectUnion
    if (UE_VERSION < 503) bool bIsUObject;
};

class FField {
    uint64_t VTable; // Implicit VTable for alignment
    FFieldClass* ClassPrivate;
    FFieldVariant Owner;
    FField* Next;
    FName NamePrivate;
    uint32_t FlagsPrivate; // EObjectFlags
};

// UProperty exists in all versions, but is only used pre-4.25
class UProperty : UField {
    int32_t ArrayDim;
    int32_t ElementSize;
    
    if (UE_VERSION < 420) uint64_t PropertyFlags;
    else EPropertyFlags PropertyFlags;
    
    uint16_t RepIndex;
    
    if (UE_VERSION >= 418) TEnumAsByte<ELifetimeCondition> BlueprintReplicationCondition;
    if (UE_VERSION < 418) FName RepNotifyFunc;
    int32_t Offset_Internal;
    if (UE_VERSION >= 414 && UE_VERSION < 418) uint32_t BlueprintReplicationCondition;
    if (UE_VERSION >= 418) FName RepNotifyFunc;
    
    UProperty* PropertyLinkNext;
    UProperty* NextRef;
    UProperty* DestructorLinkNext;
    UProperty* PostConstructLinkNext;
};

// FProperty exists from 4.25+ and replaces UProperty functionality
class FProperty : FField {
    int32_t ArrayDim;
    int32_t ElementSize;
    EPropertyFlags PropertyFlags;
    uint16_t RepIndex;
    TEnumAsByte<ELifetimeCondition> BlueprintReplicationCondition;
    int32_t Offset_Internal;
    
    if (UE_VERSION < 503) FName RepNotifyFunc;
    
    FProperty* PropertyLinkNext;
    FProperty* NextRef;
    FProperty* DestructorLinkNext;
    FProperty* PostConstructLinkNext;
    
    if (UE_VERSION >= 503) FName RepNotifyFunc;
};

// Unified type aliases - default to version-appropriate types
type ZProperty = if (UE_VERSION >= 425) FProperty else UProperty;
type ZField = if (UE_VERSION >= 425) FField else UField;

template<typename BasePropertyType = ZProperty>
class ZTObjectPropertyBase : BasePropertyType {
    UClass* PropertyClass;
};

template<typename BasePropertyType = ZProperty>
class ZTObjectProperty : ZTObjectPropertyBase<BasePropertyType> {};

template<typename BasePropertyType = ZProperty>
class ZTClassProperty : ZTObjectProperty<BasePropertyType> {
    UClass* MetaClass;
};

template<typename BasePropertyType = ZProperty>
class ZTNumericProperty : BasePropertyType {};

template<typename BasePropertyType = ZProperty>
class ZTEnumProperty : BasePropertyType {
    ZTNumericProperty<BasePropertyType>* UnderlyingProp;
    UEnum* Enum;
};

template<typename BasePropertyType = ZProperty>
class ZTByteProperty : ZTNumericProperty<BasePropertyType> {
    UEnum* Enum;
};

template<typename BasePropertyType = ZProperty>
class ZTBoolProperty : BasePropertyType {
    uint8_t FieldSize;
    uint8_t ByteOffset;
    uint8_t ByteMask;
    uint8_t FieldMask;
};

template<typename BasePropertyType = ZProperty>
class ZTArrayProperty : BasePropertyType {
    if (UE_VERSION < 503) {
        BasePropertyType* Inner;
        EArrayPropertyFlags ArrayFlags;
    } else {
        EArrayPropertyFlags ArrayFlags;
        BasePropertyType* Inner;
    }
};

template<typename BasePropertyType = ZProperty>
class ZTSetProperty : BasePropertyType {
    BasePropertyType* ElementProp;
    FScriptSetLayout SetLayout;
};

template<typename BasePropertyType = ZProperty>
class ZTMapProperty : BasePropertyType {
    BasePropertyType* KeyProp;
    BasePropertyType* ValueProp;
    FScriptMapLayout MapLayout;
    EMapPropertyFlags MapFlags;
};

template<typename BasePropertyType = ZProperty>
class ZTInterfaceProperty : BasePropertyType {
    UClass* InterfaceClass;
};

template<typename BasePropertyType = ZProperty>
class ZTSoftObjectProperty : ZTObjectPropertyBase<BasePropertyType> {};

template<typename BasePropertyType = ZProperty>
class ZTSoftClassProperty : ZTSoftObjectProperty<BasePropertyType> {
    UClass* MetaClass;
};

template<typename BasePropertyType = ZProperty>
class ZTWeakObjectProperty : ZTObjectPropertyBase<BasePropertyType> {
};

template<typename BasePropertyType = ZProperty>
class ZTLazyObjectProperty : ZTObjectPropertyBase<BasePropertyType> {
};

template<typename BasePropertyType = ZProperty>
class ZTStructProperty : BasePropertyType {
    UScriptStruct* Struct;
};

template<typename BasePropertyType = ZProperty>
class ZTDelegateProperty : BasePropertyType {
    UFunction* SignatureFunction;
};

template<typename BasePropertyType = ZProperty>
class ZTMulticastDelegateProperty : BasePropertyType {
    UFunction* SignatureFunction;
};

template<typename BasePropertyType = ZProperty>
class ZTOptionalProperty : BasePropertyType, FOptionalPropertyLayout {};

type FObjectPropertyBase = ZTObjectPropertyBase<FProperty>;
type FObjectProperty = ZTObjectProperty<FProperty>;
type FClassProperty = ZTClassProperty<FProperty>;
type FNumericProperty = ZTNumericProperty<FProperty>;
type FEnumProperty = ZTEnumProperty<FProperty>;
type FByteProperty = ZTByteProperty<FProperty>;
type FBoolProperty = ZTBoolProperty<FProperty>;
type FArrayProperty = ZTArrayProperty<FProperty>;
type FSetProperty = ZTSetProperty<FProperty>;
type FMapProperty = ZTMapProperty<FProperty>;
type FInterfaceProperty = ZTInterfaceProperty<FProperty>;
type FSoftObjectProperty = ZTSoftObjectProperty<FProperty>;
type FSoftClassProperty = ZTSoftClassProperty<FProperty>;
type FWeakObjectProperty = ZTWeakObjectProperty<FProperty>;
type FLazyObjectProperty = ZTLazyObjectProperty<FProperty>;
type FStructProperty = ZTStructProperty<FProperty>;
type FDelegateProperty = ZTDelegateProperty<FProperty>;
type FMulticastDelegateProperty = ZTMulticastDelegateProperty<FProperty>;
type FOptionalProperty = ZTOptionalProperty<FProperty>;

type UObjectPropertyBase = ZTObjectPropertyBase<UProperty>;
type UObjectProperty = ZTObjectProperty<UProperty>;
type UClassProperty = ZTClassProperty<UProperty>;
type UNumericProperty = ZTNumericProperty<UProperty>;
type UEnumProperty = ZTEnumProperty<UProperty>;
type UByteProperty = ZTByteProperty<UProperty>;
type UBoolProperty = ZTBoolProperty<UProperty>;
type UArrayProperty = ZTArrayProperty<UProperty>;
type USetProperty = ZTSetProperty<UProperty>;
type UMapProperty = ZTMapProperty<UProperty>;
type UInterfaceProperty = ZTInterfaceProperty<UProperty>;
type USoftObjectProperty = ZTSoftObjectProperty<UProperty>;
type USoftClassProperty = ZTSoftClassProperty<UProperty>;
type UWeakObjectProperty = ZTWeakObjectProperty<UProperty>;
type ULazyObjectProperty = ZTLazyObjectProperty<UProperty>;
type UStructProperty = ZTStructProperty<UProperty>;
type UDelegateProperty = ZTDelegateProperty<UProperty>;
type UMulticastDelegateProperty = ZTMulticastDelegateProperty<UProperty>;
type UOptionalProperty = ZTOptionalProperty<UProperty>;

type ZObjectPropertyBase = ZTObjectPropertyBase<>;
type ZObjectProperty = ZTObjectProperty<>;
type ZClassProperty = ZTClassProperty<>;
type ZNumericProperty = ZTNumericProperty<>;
type ZEnumProperty = ZTEnumProperty<>;
type ZByteProperty = ZTByteProperty<>;
type ZBoolProperty = ZTBoolProperty<>;
type ZArrayProperty = ZTArrayProperty<>;
type ZSetProperty = ZTSetProperty<>;
type ZMapProperty = ZTMapProperty<>;
type ZInterfaceProperty = ZTInterfaceProperty<>;
type ZSoftObjectProperty = ZTSoftObjectProperty<>;
type ZSoftClassProperty = ZTSoftClassProperty<>;
type ZWeakObjectProperty = ZTWeakObjectProperty<>;
type ZLazyObjectProperty = ZTLazyObjectProperty<>;
type ZStructProperty = ZTStructProperty<>;
type ZDelegateProperty = ZTDelegateProperty<>;
type ZMulticastDelegateProperty = ZTMulticastDelegateProperty<>;
type ZOptionalProperty = ZTOptionalProperty<>;

struct STUB {};

template<typename KeyType, typename ValueType>
struct TMap {
    uint64_t Data[10];
};

struct FRepRecord {
    uint32_t Placeholder;
};
struct FImplementedInterface {
    uint64_t Placeholder[2];
};
struct FGCReferenceTokenStream {
    uint64_t Placeholder1[2]; // TArray Tokens
    if (UE_VERSION >= 500 && UE_VERSION < 502) {
        uint64_t Placeholder2; // StackSize + TokenType in UE 5.0-5.1
    }
};
struct FWindowsCriticalSection {
    uint64_t Placeholder[5];
};
struct FWindowsRWLock {
    uint64_t Placeholder;
};
struct FNativeFunctionLookup {
    uint64_t Placeholder[2];
};
struct alignas(16) FTokenStreamOwner {
    uint64_t Placeholder[4];
};

// FUObjectArray related structures
struct FThreadSafeCounter {
    uint32_t Value;
};

struct FUObjectItem {
    UObject* Object;
    
    if (UE_VERSION < 413) int32_t ClusterAndFlags;
    if (UE_VERSION >= 413) int32_t Flags;
    if (UE_VERSION >= 413 && UE_VERSION < 416) int32_t ClusterIndex;
    if (UE_VERSION >= 416) int32_t ClusterRootIndex;
    
    int32_t SerialNumber;
    if (UE_VERSION >= 505) int32_t RefCount;
};

struct FFixedUObjectArray {
    FUObjectItem* Objects;
    int32_t MaxElements;
    int32_t NumElements;
};

struct FChunkedFixedUObjectArray {
    (FUObjectItem*)* Objects; // Array of pointers to FUObjectItem chunks
    FUObjectItem* PreAllocatedObjects;
    int32_t MaxElements;
    int32_t NumElements;
    int32_t MaxChunks;
    int32_t NumChunks;
};

template<int Size>
struct FPaddingForCacheContention {
    uint8_t Padding[Size];
};

template<typename T, int PadSize>
struct TLockFreePointerListUnordered {
    if (UE_VERSION < 417) {
        STUB* Ptr;
    } else {
        FPaddingForCacheContention<PadSize> PadToAvoidContention1;
        uint64_t Head; // FIndexedPointer Head
        FPaddingForCacheContention<PadSize> PadToAvoidContention2;
    }
};
type EFunctionFlags = uint32_t;

class FOutputDevice {
    uint64_t VTable; // Implicit VTable
    bool bSuppressEventTag;
    bool bAutoEmitLineTerminator;
};

struct FOutParmRec {
    ZProperty* Property;
    uint8_t* PropAddr;
    FOutParmRec* NextOutParm;
};

struct FFrame : FOutputDevice {
    UFunction* Node;
    UObject* Object;
    uint8_t* Code;
    uint8_t* Locals;
    
    ZProperty* MostRecentProperty;
    
    uint8_t* MostRecentPropertyAddress;
    
    // Extra field added in UE 5.1 and 5.3+ (but not 5.2)
    if ((UE_VERSION == 501) || (UE_VERSION >= 503)) {
        uint8_t* MostRecentPropertyContainer;
    }
    
    // FlowStack array - type changes but size stays same (0x30 bytes)
    uint64_t FlowStack[6]; // Placeholder for TArray variants
    
    FFrame* PreviousFrame;
    FOutParmRec* OutParms;
    
    ZField* PropertyChainForCompiledIn;
    
    UFunction* CurrentNativeFunction;
    
    // Extra fields added in UE 5.1 and 5.3+ (but not 5.2)
    if ((UE_VERSION == 501) || (UE_VERSION >= 503)) {
        FFrame* PreviousTrackingFrame;
    }
    
    bool bArrayContextFailed;
    
    // Extra field added in UE 5.1 and 5.3+ (but not 5.2)
    if ((UE_VERSION == 501) || (UE_VERSION >= 503)) {
        bool bAbortingExecution;
    }
};

struct FScriptSparseArrayLayout {
    if (UE_VERSION < 422) int32_t ElementOffset;
    int32_t Alignment;
    int32_t Size;
};

struct FScriptSetLayout {
    if (UE_VERSION < 422) int32_t ElementOffset;
    int32_t HashNextIdOffset;
    int32_t HashIndexOffset;
    int32_t Size;
    FScriptSparseArrayLayout SparseArrayLayout;
};

struct FScriptMapLayout {
    if (UE_VERSION < 422) int32_t KeyOffset;
    int32_t ValueOffset;
    FScriptSetLayout SetLayout;
};

struct FOptionalPropertyLayout {
    FProperty* ValueProperty;
};

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
    UFunction* EventGraphFunction;
    int32_t EventGraphCallOffset;
    void* Func;
};

type EStructFlags = uint32_t;

struct ICppStructOps {
    uint64_t Placeholder[4];
};

class UScriptStruct : UStruct {
    EStructFlags StructFlags;
    if (UE_VERSION < 414) bool bCppStructOpsFromBaseClass;
    bool bPrepareCppStructOpsCompleted;
    ICppStructOps* CppStructOps;
};

struct FString : TArray<wchar_t> {};

template<typename A, typename B>
struct TTuple {
    A First;
    B Second;
};

struct FText {
    uint64_t Placeholder[3];
};

type EEnumFlags = if (UE_VERSION >= 505) uint8_t else uint32_t;
type ECppForm = if (UE_VERSION >= 505) uint8_t else uint32_t;

struct UEnumNameTuple {
    type ValueType = if (UE_VERSION < 415) uint8_t else int64_t;

    FName Name;
    ValueType Value;
};

class UEnum : UField {
    FString CppType;
    
    type ValueType = if (UE_VERSION < 415) uint8_t else int64_t;
    TArray<UEnumNameTuple> Names;
    
    ECppForm CppForm;
    
    if (UE_VERSION >= 426) EEnumFlags EnumFlags;
    if (UE_VERSION >= 505) FName EnumPackage;
    if (UE_VERSION >= 415) STUB* EnumDisplayNameFn;
    if (UE_VERSION >= 501 && UE_VERSION != 502 && UE_VERSION < 505) FName EnumPackage;
};

struct ICppClassTypeInfo {};
struct FUObjectCppClassStaticFunctions {
    uint64_t Placeholder;
};

// FUObjectArray listener interface placeholders
class FUObjectCreateListener {
    uint64_t VTable;
};

class FUObjectDeleteListener {
    uint64_t VTable;
};

// FUObjectArray core structure
struct FUObjectArray {
    int32_t ObjFirstGCIndex;
    int32_t ObjLastNonGCIndex;
    int32_t MaxObjectsNotConsideredByGC;
    bool OpenForDisregardForGC;
    
    // Object storage evolved from fixed to chunked arrays
    if (UE_VERSION < 420) FFixedUObjectArray ObjObjects;
    else FChunkedFixedUObjectArray ObjObjects;
    
    FWindowsCriticalSection ObjObjectsCritical;
    
    // Available list type changed significantly across versions
    if (UE_VERSION < 422) TLockFreePointerListUnordered<int, 128> ObjAvailableList;
    else if (UE_VERSION < 427) TLockFreePointerListUnordered<int, 64> ObjAvailableList;
    else TArray<int> ObjAvailableList;
    
    TArray<FUObjectCreateListener*> UObjectCreateListeners;
    TArray<FUObjectDeleteListener*> UObjectDeleteListeners;
    
    FWindowsCriticalSection UObjectDeleteListenersCritical;
    
    // Serial number field name changed in UE 5.1 and 5.3+, but back to Master in 5.2
    if (UE_VERSION == 501 || UE_VERSION >= 503) FThreadSafeCounter PrimarySerialNumber;
    else FThreadSafeCounter MasterSerialNumber;
    
    // New field added in UE 5.3+
    if (UE_VERSION >= 503) bool bShouldRecycleObjectIndices;
};


type EObjectFlags = int32_t;
type EClassFlags = int32_t;
type EClassCastFlags = uint64_t;

class UObject {
    uint64_t VTable;
    EObjectFlags ObjectFlags;
    int32_t InternalIndex;
    UClass* ClassPrivate;
    FName NamePrivate;
    UObject* OuterPrivate;
};

class UField : UObject {
    UField* Next;
};
class UStruct : UField {
    // FStructBaseChain becomes base class in 4.22+ but Gospel shows it as member (intentional difference)
    if (UE_VERSION >= 422) FStructBaseChain BaseChain;
    
    UStruct* SuperStruct;
    UField* Children;
    if (UE_VERSION >= 425) FField* ChildProperties;
    int32_t PropertiesSize;
    int32_t MinAlignment;
    TArray<uint8_t> Script;
    
    ZProperty* PropertyLink;
    ZProperty* RefLink;
    ZProperty* DestructorLink;
    ZProperty* PostConstructLink;
    
    // Object references array - name changes in 4.25+
    if (UE_VERSION >= 425) TArray<UObject*> ScriptAndPropertyObjectReferences;
    else TArray<UObject*> ScriptObjectReferences;
    
    // Additional fields in 4.25+
    if (UE_VERSION >= 425) {
        STUB* UnresolvedScriptProperties;
        if (UE_VERSION >= 500) STUB* UnversionedGameSchema;
        else STUB* UnversionedSchema;
    }
};
class UClass : UStruct {
    if (UE_VERSION < 414) FFastIndexingClassTreeRegistrar FastIndexingRegistrar;
    else if (UE_VERSION < 422) FClassBaseChain ClassBaseChain;

    STUB* ClassConstructor;
    STUB* ClassVTableHelperCtorCaller;
    
    if (UE_VERSION >= 501) STUB* CppClassStaticFunctions; // FUObjectCppClassStaticFunctions
    else STUB* ClassAddReferencedObjects; // UE < 5.1
    
    // ClassUnique format varies
    if (UE_VERSION >= 500) {
        int32_t ClassUnique;
        int32_t FirstOwnedClassRep;
        bool bCooked;
        bool bLayoutChanging;
    } else if (UE_VERSION >= 418) {
        uint32_t ClassUnique : 1;
        uint32_t bCooked : 1;
    } else int32_t ClassUnique;
    
    EClassFlags ClassFlags;
    EClassCastFlags ClassCastFlags;
    UClass* ClassWithin;
    
    if (UE_VERSION < 500) UObject* ClassGeneratedBy;
    if (UE_VERSION == 421) UStructProperty* UberGraphFramePointerProperty;
    FName ClassConfigName;
    if (UE_VERSION < 418) bool bCooked;
    TArray<FRepRecord> ClassReps;
    TArray<UField*> NetFields;
    if (UE_VERSION >= 425 && UE_VERSION < 500) int32_t FirstOwnedClassRep;
    UObject* ClassDefaultObject;
    if (UE_VERSION >= 416 && UE_VERSION < 418) ICppClassTypeInfo* CppTypeInfo;
    if (UE_VERSION >= 424) {
        STUB* SparseClassData;
        UScriptStruct* SparseClassDataStruct;
    }
    TMap<FName, UFunction*> FuncMap;
    if (UE_VERSION < 418) TMap<FName, UFunction*> ParentFuncMap;
    if (UE_VERSION >= 412 && UE_VERSION < 418) TMap<FName, UFunction*> InterfaceFuncMap;
    if (UE_VERSION >= 502) FWindowsRWLock FuncMapLock;
    if (UE_VERSION >= 503) {
        TMap<FName, UFunction*> AllFunctionsCache;
        FWindowsRWLock AllFunctionsCacheLock;
    } else if (UE_VERSION >= 418) {
        TMap<FName, UFunction*> SuperFuncMap;
        if (UE_VERSION >= 421) {
            FWindowsRWLock SuperFuncMapLock;
        }
    }
    TArray<FImplementedInterface> Interfaces;
    if (UE_VERSION >= 503) STUB* ReferenceSchema; // UE::GC::FSchemaOwner
    else if (UE_VERSION >= 502) FTokenStreamOwner ReferenceTokens; // UE::GC::FTokenStreamOwner
    else {
        FGCReferenceTokenStream ReferenceTokenStream;
        if (UE_VERSION >= 415 && UE_VERSION < 502) {
            FWindowsCriticalSection ReferenceTokenStreamCritical;
        }
    }
    
    TArray<FNativeFunctionLookup> NativeFunctionLookupTable;
};
