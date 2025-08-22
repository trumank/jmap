input int UE_VERSION;
input int WITH_CASE_PRESERVING_NAME = 0;

/*
 * VERIFIED CROSS-VERSION COMPATIBLE TYPES
 * - UObject: Base UObject hierarchy with flattened UObjectBase fields
 * - UStruct
 * - UClass
 * - UScriptStruct
 * - UFunction
 * - UEnum
 * - FUObjectArray
 * - UProperty
 * - FProperty
 * - FStaticConstructObjectParameters 
 * - FFrame
 * 
 * Test any type: python3 verify_layout.py <TypeName>
 */

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

type FVector = if (UE_VERSION >= 501) TVector<double> else TVector<float>;

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
struct alignas(if (UE_VERSION >= 411 && UE_VERSION < 422) 8 else 4) FName {
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
    void* ConstructFn; // FField*(*)(const FFieldVariant*, const FName*, EObjectFlags)
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
    
    if (UE_VERSION < 418) {
        FName RepNotifyFunc;
        int32_t Offset_Internal;
        if (UE_VERSION >= 414) uint32_t BlueprintReplicationCondition;
    } else {
        TEnumAsByte<ELifetimeCondition> BlueprintReplicationCondition;
        int32_t Offset_Internal;
        FName RepNotifyFunc;
    }
    
    UProperty* PropertyLinkNext;
    UProperty* NextRef;
    UProperty* DestructorLinkNext;
    UProperty* PostConstructLinkNext;
    if (UE_VERSION == 409) UProperty* RollbackLinkNext;
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

template<typename ElementType>
struct TSet {
    uint64_t Data[8];
};

struct FRepRecord {
    uint32_t Placeholder;
};
struct FImplementedInterface {
    uint64_t Placeholder[2];
};
struct FGCReferenceTokenStream {
    uint64_t Placeholder1[2]; // TArray Tokens
    if (UE_VERSION == 501) {
        uint64_t Placeholder2; // StackSize + TokenType in UE 5.1 only
    }
};
struct FWindowsCriticalSection {
    uint64_t Placeholder[5];
};
type FCriticalSection = FWindowsCriticalSection;
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
    if (UE_VERSION >= 408) UFunction* EventGraphFunction;
    if (UE_VERSION >= 408) int32_t EventGraphCallOffset;
    void* Func;
};

type EStructFlags = uint32_t;

struct ICppStructOps {
    uint64_t Placeholder[4];
};

class UScriptStruct : UStruct {
    EStructFlags StructFlags;
    if (UE_VERSION < 408) ICppStructOps* CppStructOps;
    if (UE_VERSION < 414) bool bCppStructOpsFromBaseClass;
    bool bPrepareCppStructOpsCompleted;
    if (UE_VERSION >= 408) ICppStructOps* CppStructOps;
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
    if (UE_VERSION >= 409) ValueType Value;
};

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

template<typename ElementType, int MaxTotalElements, int ElementsPerChunk>
class TStaticIndirectArrayThreadSafeRead  {
    ((ElementType*)*) Chunks[MaxTotalElements / ElementsPerChunk];
    int32_t NumElements;
    int32_t NumChunks;
};

type FUObjectArrayOld = TStaticIndirectArrayThreadSafeRead<UObject, 8 * 1024 * 1024, 16 * 1024>;
type FUObjectArrayOlder = TArray<UObject*>;

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
    if (UE_VERSION >= 411) int32_t MaxObjectsNotConsideredByGC;
    if (UE_VERSION < 411) int32_t OpenForDisregardForGC;
    else bool OpenForDisregardForGC;
    
    if          (UE_VERSION == 407) FUObjectArrayOlder ObjObjects;
    else if     (UE_VERSION <  411) FUObjectArrayOld ObjObjects;
    else if     (UE_VERSION <  420) FFixedUObjectArray ObjObjects;
    else                            FChunkedFixedUObjectArray ObjObjects;
    
    if          (UE_VERSION == 407) TArray<int> ObjAvailable;
    else {
        if      (UE_VERSION >= 408) FCriticalSection ObjObjectsCritical;
        if      (UE_VERSION <  422) TLockFreePointerListUnordered<int, 128> ObjAvailableList;
        else if (UE_VERSION <  427) TLockFreePointerListUnordered<int, 64> ObjAvailableList;
        else                        TArray<int> ObjAvailableList;
    }
    
    TArray<FUObjectCreateListener*> UObjectCreateListeners;
    TArray<FUObjectDeleteListener*> UObjectDeleteListeners;
    
    if (UE_VERSION >= 409) FCriticalSection UObjectDeleteListenersCritical;
    
    if (UE_VERSION >= 411) {
        if (UE_VERSION >= 501) FThreadSafeCounter PrimarySerialNumber;
        else FThreadSafeCounter MasterSerialNumber;
    }
    
    if (UE_VERSION >= 502) bool bShouldRecycleObjectIndices;
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
        if (UE_VERSION <= 500) STUB* UnversionedSchema;
        else STUB* UnversionedGameSchema;
    }
};
class UClass : UStruct,
    FFastIndexingClassTreeRegistrar if (UE_VERSION >= 408 && UE_VERSION < 414),
    FClassBaseChain if (UE_VERSION >= 414 && UE_VERSION < 422) {

    STUB* ClassConstructor;
    if (UE_VERSION >= 408) STUB* ClassVTableHelperCtorCaller;
    
    if (UE_VERSION == 500) STUB* ClassAddReferencedObjects;
    else if (UE_VERSION >= 501) STUB* CppClassStaticFunctions;
    else STUB* ClassAddReferencedObjects;

    if (UE_VERSION >= 408 && UE_VERSION < 418 || UE_VERSION >= 501) uint32_t ClassUnique;
    if (UE_VERSION >= 501) {
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
    
    if (UE_VERSION < 501) UObject* ClassGeneratedBy;
    if (UE_VERSION == 421) UStructProperty* UberGraphFramePointerProperty;
    FName ClassConfigName;
    if (UE_VERSION >= 408 && UE_VERSION < 418) bool bCooked;
    TArray<FRepRecord> ClassReps;
    TArray<UField*> NetFields;
    if (UE_VERSION >= 425 && UE_VERSION < 501) int32_t FirstOwnedClassRep;
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

type EInternalObjectFlags = int32_t;

struct FGuid {
    uint32_t A;
    uint32_t B;
    uint32_t C;
    uint32_t D;
};

struct FLinkerLoad {
    uint64_t Placeholder[50];
};

struct FCustomVersionContainer {
    if (UE_VERSION >= 410 && UE_VERSION < 419) uint64_t Placeholder[10];
    else if (UE_VERSION >= 419) uint64_t Placeholder[2];
    else uint64_t Placeholder[2];
};

struct FObjectThumbnail {
    uint64_t Placeholder[10];
};

template<typename T>
struct TScopedPointer {
    T* Ptr;
};

template<typename T>
struct TUniquePtr {
    T* Ptr;
};

struct FIntPoint {
    int32_t X;
    int32_t Y;
};

struct FIntVector {
    int32_t X;
    int32_t Y;
    int32_t Z;
};

struct FBox {
    FVector Min;
    FVector Max;
    uint8_t IsValid;
};

struct FWorldTileLODInfo {
    int32_t RelativeStreamingDistance;
    float Reserved0;
    float Reserved1;
    int32_t Reserved2;
    int32_t Reserved3;
};

struct FWorldTileLayer {
    FString Name;
    int32_t Reserved0;
    FIntPoint Reserved1;
    int32_t StreamingDistance;
    bool DistanceStreamingEnabled;
};

struct FWorldTileInfo {
    if (UE_VERSION >= 420) {
        FIntVector Position;
        FIntVector AbsolutePosition;
    } else {
        FIntPoint Position;
        FIntPoint AbsolutePosition;
    }
    
    FBox Bounds;
    
    FWorldTileLayer Layer;
    
    if (UE_VERSION >= 408) bool bHideInTileView;
    else bool Reserved0;
    
    FString ParentTilePackageName;
    TArray<FWorldTileLODInfo> LODList;
    int32_t ZOrder;
};

struct UMetaData {
    uint64_t Placeholder[10];
};

struct FPackageId {
    uint64_t Value;
};

struct FPackagePath {
    if (UE_VERSION >= 501) {
        FName PackageName;
        uint32_t HeaderExtension; // EPackageExtension (assuming uint32_t enum)
    } else {
        uint64_t StringData; // TUniquePtr placeholder  
        uint16_t PathDataLen;
        uint16_t PackageNameRootLen;
        uint16_t FilePathRootLen;
        uint16_t ExtensionLen;
        uint8_t IdType; // FPackagePath::EPackageIdType
        uint8_t HeaderExtension; // EPackageExtension
    }
};

struct FPackageFileVersion {
    int32_t FileVersionUE4;
    int32_t FileVersionUE5;
};

struct FObjectInstancingGraph {
    UObject* SourceRoot;
    UObject* DestinationRoot;
    
    if (UE_VERSION >= 503) {
        uint32_t InstancingOptions; // EObjectInstancingGraphOptions
        bool bCreatingArchetype;
        bool bLoadingObject;
        if (UE_VERSION >= 506) bool bCanUseDynamicInstancing;
    } else {
        bool bCreatingArchetype;
        if (UE_VERSION < 503) bool bEnableSubobjectInstancing;
        bool bLoadingObject;
    }
    
    TMap<UObject*, UObject*> SourceToDestinationMap;
    
    if (UE_VERSION >= 417 && UE_VERSION < 427 || UE_VERSION == 500) {
        TMap<UObject*, UObject*> ReplaceMap;
    } else if (UE_VERSION >= 501) {
        TSet<FProperty*> SubobjectInstantiationExclusionList;
    }
};

struct TFunction {
    if (UE_VERSION >= 506) uint64_t Placeholder[6];
    else uint64_t Placeholder[8];
};

struct FObjectInitializerOverrides {
    TArray<STUB> Overrides; // TArray<FObjectInitializer::FOverrides::FOverride, VariousAllocators>
};

class UPackage : UObject {
    if (UE_VERSION >= 419) {
        bool bDirty : 1;
        bool bHasBeenFullyLoaded : 1;
        if (UE_VERSION >= 425) bool bCanBeImported : 1;
    } else {
        bool bDirty;
        bool bHasBeenFullyLoaded;
        if (UE_VERSION < 418) bool bShouldFindExportsInMemoryFirst;
    }
    
    if (UE_VERSION >= 505) {
        uint32_t PackageFlagsPrivate;
        FPackageId PackageId;
        FPackagePath LoadedPath;
        TUniquePtr<STUB> AdditionalInfo; // UPackage::FAdditionalInfo
    } else if (UE_VERSION >= 501) {
        if (UE_VERSION < 504) FGuid Guid;
        uint32_t PackageFlagsPrivate;
        FPackageId PackageId;
        FPackagePath LoadedPath;
        FPackageFileVersion LinkerPackageVersion;
        int32_t LinkerLicenseeVersion;
        FCustomVersionContainer LinkerCustomVersion;
        FLinkerLoad* LinkerLoad;
        uint64_t FileSize;
        FName FileName;
        TUniquePtr<FWorldTileInfo> WorldTileInfo;
    } else {
        if (UE_VERSION < 419) FName FolderName;
        float LoadTime;
        FGuid Guid;
        TArray<int> ChunkIDs;
        if (UE_VERSION <= 418) FName ForcedExportBasePackageName;
        if (UE_VERSION >= 410 && UE_VERSION < 419) uint32_t* PackageFlagsPrivate;
        
        if (UE_VERSION < 419) uint32_t PackageFlags;
        else uint32_t PackageFlagsPrivate;

        if (UE_VERSION >= 425) FPackageId PackageId;
        if (UE_VERSION >= 500) FPackagePath LoadedPath;
        if (UE_VERSION >= 419) int32_t PIEInstanceID;
        FName FileName;
        
        if (UE_VERSION >= 408) {
            FLinkerLoad* LinkerLoad;
            int32_t LinkerPackageVersion;
            int32_t LinkerLicenseeVersion;
            FCustomVersionContainer LinkerCustomVersion;
        }
        
        uint64_t FileSize;
        
        if (UE_VERSION >= 407 && UE_VERSION < 415) TScopedPointer<TMap<FName, FObjectThumbnail>> ThumbnailMap;
        else if (UE_VERSION < 419) TUniquePtr<TMap<FName, FObjectThumbnail>> ThumbnailMap;
        
        if (UE_VERSION >= 407 && UE_VERSION < 419) UMetaData* MetaData;
        
        if (UE_VERSION >= 407 && UE_VERSION < 415) TScopedPointer<FWorldTileInfo> WorldTileInfo;
        else TUniquePtr<FWorldTileInfo> WorldTileInfo;
        
        if (UE_VERSION >= 412 && UE_VERSION < 420) TMap<FName, int> ClassUniqueNameIndexMap;
        if (UE_VERSION >= 414 && UE_VERSION < 419) int32_t PIEInstanceID;
    }
};

struct FStaticConstructObjectParameters {
    UClass* Class;
    UObject* Outer;
    FName Name;
    EObjectFlags SetFlags;
    EInternalObjectFlags InternalSetFlags;
    bool bCopyTransientsFromClassDefaults;
    bool bAssumeTemplateIsArchetype;
    UObject* Template;
    FObjectInstancingGraph* InstanceGraph;
    UPackage* ExternalPackage;
    
    if (UE_VERSION == 500) FObjectInitializerOverrides* SubobjectOverrides;
    
    if (UE_VERSION >= 501) {
        TFunction PropertyInitCallback;
        if (UE_VERSION >= 506) int32_t SerialNumber;
        FObjectInitializerOverrides* SubobjectOverrides;
    }
};
