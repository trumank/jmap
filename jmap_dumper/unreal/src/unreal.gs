import unreal::core::{UE_VERSION, WITH_CASE_PRESERVING_NAME, int8_t, uint8_t, int16_t, uint16_t, int32_t, uint32_t, int64_t, uint64_t};
import unreal::containers::{FScriptElement, TSizedHeapAllocator, TArray, TMap, TSet};
import unreal::properties::{
    EPropertyFlags, ELifetimeCondition, EArrayPropertyFlags, EMapPropertyFlags,
    TEnumAsByte, TObjectPtr,
    FFieldClass, FFieldVariant, FField,
    ZFieldBase, ZField, ZProperty,
    FScriptSparseArrayLayout, FScriptSetLayout, FScriptMapLayout, FOptionalPropertyLayout,
    ZObjectPropertyBase, ZObjectProperty, ZClassProperty, ZNumericProperty,
    ZEnumProperty, ZByteProperty, ZBoolProperty,
    ZArrayProperty, ZSetProperty, ZMapProperty,
    ZInterfaceProperty, ZSoftObjectProperty, ZSoftClassProperty,
    ZWeakObjectProperty, ZLazyObjectProperty, ZStructProperty,
    ZDelegateProperty, ZMulticastDelegateProperty, ZMulticastSparseDelegateProperty,
    ZOptionalProperty
};
import unreal::objects::{
    EObjectFlags, EClassFlags, EClassCastFlags, EFunctionFlags, EStructFlags,
    FStructBaseChain, FClassBaseChain, FFastIndexingClassTreeRegistrar,
    FOutputDevice, FOutParmRec, FFrame,
    UObject, UField, UStruct, UFunction, UScriptStruct, UEnum, UClass,
    ICppStructOps, ICppClassTypeInfo, FUObjectCppClassStaticFunctions,
    UEnumNameTuple, EEnumFlags, ECppForm
};
import unreal::uobjectarray::{
    FThreadSafeCounter, FUObjectItem, FFixedUObjectArray, FChunkedFixedUObjectArray,
    FPaddingForCacheContention, TLockFreePointerListUnordered,
    TStaticIndirectArrayThreadSafeRead, FUObjectArrayOld, FUObjectArrayOlder,
    FUObjectCreateListener, FUObjectDeleteListener, FUObjectArray
};
import unreal::archive::{
    FEngineVersionBase, FEngineVersion, FFastPathLoadBuffer,
    FArchiveState, FArchive, FArchiveUObject,
    FStructuredArchive, FBinaryArchiveFormatter, FAsyncPackage
};
import unreal::package::{
    FPackageIndex, FLinkerTables, FPackageFileVersion, FPackageFileSummary,
    FGatherableTextData, FSHA1, ELinkerType, FLinker, FLinkerInstancingContext,
    TOptional, FLinkerLoad, FPackageId, FPackagePath, UPackage
};

/* Test block comment */
template<typename T>
struct TVector {
    T X;
    T Y;
    T Z;
};

type FVector = if (UE_VERSION >= 500) TVector<double> else TVector<float>;

/// TEST
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

struct STUB {};

struct FRepRecord {
    uint32_t Placeholder;
};
struct FImplementedInterface {
    uint64_t Placeholder[2];
};
struct FGCReferenceTokenStream {
    uint64_t Placeholder1[2]; // TArray Tokens
    if (UE_VERSION >= 500 && UE_VERSION < 502) {
        uint64_t Placeholder2; // StackSize + TokenType in UE 5.0-5.1 only
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


struct FString : TArray<wchar_t> {};

template<typename A, typename B>
struct TTuple {
    A First;
    B Second;
};

struct FText {
    uint64_t Placeholder[3];
};

type EInternalObjectFlags = int32_t;

/// TEST
struct FGuid {
    uint32_t A;
    uint32_t B;
    uint32_t C;
    uint32_t D;
};

/// TEST
struct FIoHash {
    uint8_t Hash[20];
};


/// TEST
struct FCustomVersion {
    FGuid Key;
    int32_t Version;
    if (UE_VERSION < 410) FString FriendlyName;
    int32_t ReferenceCount;
    if (UE_VERSION >= 426) void* Validator;
    if (UE_VERSION >= 410) FName FriendlyName;
};

/// TEST
struct FCustomVersionContainer {
    if (UE_VERSION >= 410 && UE_VERSION < 419) TSet<FCustomVersion> Versions;
    else TArray<FCustomVersion> Versions;
};

/// TEST
struct FStaticCustomVersionRegistry {
    FWindowsRWLock Lock;
    FCustomVersionContainer Registered;
    // TMap with inline allocation - very large
    if (UE_VERSION >= 426) uint64_t Queue[408];
    else uint64_t Queue[344];
};

/// TEST
struct FObjectThumbnail {
    int32_t ImageWidth;
    int32_t ImageHeight;
    TArray<uint8_t> CompressedImageData;
    TArray<uint8_t> ImageData;
    bool bIsDirty;
    bool bLoadedFromDisk;
    if (UE_VERSION >= 500) bool bIsJPEG;
    bool bCreatedAfterCustomThumbForSharedTypesEnabled;
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

/// TEST
struct FBox {
    FVector Min;
    FVector Max;
    uint8_t IsValid;
};

/// TEST
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

/// TEST
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

/// TEST
class UMetaData : UObject {
    TMap<STUB, TMap<FName, FString>> ObjectMetaDataMap;
    if (UE_VERSION >= 413 && UE_VERSION < 506) TMap<FName, FString> RootMetaDataMap;
};

/// TEST
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
    
    if (UE_VERSION >= 417 && UE_VERSION < 427) {
        TMap<UObject*, UObject*> ReplaceMap;
    } else if (UE_VERSION >= 501) {
        TSet<ZProperty*> SubobjectInstantiationExclusionList;
    }
};

struct alignas(16) TFunction {
    if (UE_VERSION >= 506) uint64_t Placeholder[6]; // 48 bytes for UE 5.6+
    else uint64_t Placeholder[8]; // 64 bytes for earlier versions
};

/// TEST
struct FObjectInitializerOverrides {
    TArray<STUB> Overrides; // TArray<FObjectInitializer::FOverrides::FOverride, VariousAllocators>
};

/// TEST
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
    
    if (UE_VERSION >= 500) TFunction PropertyInitCallback;
    if (UE_VERSION >= 506) int32_t SerialNumber;
    if (UE_VERSION >= 500) FObjectInitializerOverrides* SubobjectOverrides;
};
