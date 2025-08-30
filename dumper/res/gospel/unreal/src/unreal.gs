input int UE_VERSION;
input bool WITH_CASE_PRESERVING_NAME = false;

// Definitions for types with explicit sizes
type int8_t = char;
type uint8_t = unsigned char;
type int16_t = short int;
type uint16_t = unsigned short int;
type int32_t = int;
type uint32_t = unsigned int;
type int64_t = long long int;
type uint64_t = unsigned long long int;

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

/// TEST
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

// Unified type aliases - default to version-appropriate types
type ZFieldBase = if (UE_VERSION >= 425) FField else UField;
struct ZField : ZFieldBase {};

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

struct STUB {};

template<typename KeyType, typename ValueType>
struct TMap {
    uint64_t Data[10];
};

template<typename ElementType>
struct TSet {
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

/// TEST
struct FThreadSafeCounter {
    uint32_t Counter;
};

/// TEST
struct FUObjectItem {
    UObject* Object;
    
    if (UE_VERSION < 413) int32_t ClusterAndFlags;
    if (UE_VERSION >= 413) int32_t Flags;
    if (UE_VERSION >= 413 && UE_VERSION < 416) int32_t ClusterIndex;
    if (UE_VERSION >= 416) int32_t ClusterRootIndex;
    
    int32_t SerialNumber;
    if (UE_VERSION >= 505) int32_t RefCount;
};

/// TEST
struct FFixedUObjectArray {
    FUObjectItem* Objects;
    int32_t MaxElements;
    int32_t NumElements;
};

/// TEST
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

type EStructFlags = uint32_t;

struct ICppStructOps {
    uint64_t Placeholder[4];
};

/// TEST
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

template<typename ElementType, int MaxTotalElements, int ElementsPerChunk>
class TStaticIndirectArrayThreadSafeRead  {
    ((ElementType*)*) Chunks[MaxTotalElements / ElementsPerChunk];
    int32_t NumElements;
    int32_t NumChunks;
};

type FUObjectArrayOld = TStaticIndirectArrayThreadSafeRead<UObject, 8 * 1024 * 1024, 16 * 1024>;
type FUObjectArrayOlder = TArray<UObject*>;

/// FUObjectArray listener interface placeholders
/// TEST
class FUObjectCreateListener {
    uint64_t VTable;
};

/// TEST
class FUObjectDeleteListener {
    uint64_t VTable;
};

/// FUObjectArray core structure
/// TEST
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
        if (UE_VERSION <= 500) STUB* UnversionedSchema;
        else STUB* UnversionedGameSchema;
    }
};
/// TEST
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
    if (UE_VERSION == 421) ZStructProperty* UberGraphFramePointerProperty;
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
struct FLinkerTables {
    TArray<STUB> ImportMap; // TArray<FObjectImport>
    TArray<STUB> ExportMap; // TArray<FObjectExport>
    if (UE_VERSION >= 506) {
        TArray<STUB> CellImportMap; // TArray<FCellImport>
        TArray<STUB> CellExportMap; // TArray<FCellExport>
    }
    TArray<STUB> DependsMap; // TArray<TArray<FPackageIndex>>
    if (UE_VERSION < 415) {
        TArray<FString> StringAssetReferencesMap;
    } else if (UE_VERSION < 418) {
        TArray<FString> StringAssetReferencesMap;
        TMap<STUB, STUB> SearchableNamesMap; // TMap<FPackageIndex, TArray<FName>>
    } else {
        TArray<FName> SoftPackageReferenceList;
        TMap<STUB, STUB> SearchableNamesMap; // TMap<FPackageIndex, TArray<FName>>
    }
};

/// TEST
struct FPackageFileSummary {
    uint32_t Tag;
    if (UE_VERSION < 501) {
        int32_t FileVersionUE4;
        int32_t FileVersionLicenseeUE4;
    } else {
        FPackageFileVersion FileVersionUE;
        int32_t FileVersionLicenseeUE;
    }
    FCustomVersionContainer CustomVersionContainer;
    if (UE_VERSION < 500) {
        int32_t TotalHeaderSize;
        uint32_t PackageFlags;
    } else {
        uint32_t PackageFlags;
        int32_t TotalHeaderSize;
    }
    if (UE_VERSION >= 501) FString PackageName;
    else FString FolderName;
    int32_t NameCount;
    int32_t NameOffset;
    if (UE_VERSION >= 501) {
        int32_t SoftObjectPathsCount;
        int32_t SoftObjectPathsOffset;
    }
    if (UE_VERSION >= 419) FString LocalizationId;
    if (UE_VERSION >= 409) {
        int32_t GatherableTextDataCount;
        int32_t GatherableTextDataOffset;
    }
    if (UE_VERSION >= 506) int32_t MetaDataOffset;
    int32_t ExportCount;
    int32_t ExportOffset;
    int32_t ImportCount;
    int32_t ImportOffset;
    if (UE_VERSION >= 506) {
        int32_t CellExportCount;
        int32_t CellExportOffset;
        int32_t CellImportCount;
        int32_t CellImportOffset;
    }
    int32_t DependsOffset;
    
    // String/Soft Package References evolution
    if (UE_VERSION < 418) {
        int32_t StringAssetReferencesCount;
        int32_t StringAssetReferencesOffset;
    } else {
        int32_t SoftPackageReferencesCount;
        int32_t SoftPackageReferencesOffset;
    }
    
    if (UE_VERSION >= 415) int32_t SearchableNamesOffset;
    int32_t ThumbnailTableOffset;
    if (UE_VERSION < 506) FGuid Guid;
    if (UE_VERSION >= 506) FIoHash SavedHash;
    
    // Generations and Engine version - these come right after Guid in memory layout
    TArray<int32_t> Generations;
    
    if (UE_VERSION >= 408) {
        FEngineVersion SavedByEngineVersion;
        FEngineVersion CompatibleWithEngineVersion;
    } else {
        FEngineVersion EngineVersion;
    }
    
    // Compression flags
    if (UE_VERSION >= 500) uint32_t CompressionFlags;
    else uint32_t CompressionFlags;
    
    // Compressed chunks and package source
    uint32_t PackageSource;
    if (UE_VERSION >= 407 && UE_VERSION <= 417) TArray<uint64_t> CompressedChunks; // TArray<FCompressedChunk>
    if (UE_VERSION >= 407 && UE_VERSION <= 416) TArray<FString> AdditionalPackagesToCook;
    if (UE_VERSION >= 407) bool bUnversioned;
    if (UE_VERSION >= 407 && UE_VERSION <= 413) uint64_t TextureAllocations[4]; // FTextureAllocations (32 bytes)
    if (UE_VERSION >= 407) int32_t AssetRegistryDataOffset;
    if (UE_VERSION >= 407) int64_t BulkDataStartOffset;
    if (UE_VERSION >= 407) int32_t WorldTileInfoDataOffset;
    if (UE_VERSION >= 407) TArray<int32_t> ChunkIDs;
    if (UE_VERSION >= 414) int32_t PreloadDependencyCount;
    if (UE_VERSION >= 414) int32_t PreloadDependencyOffset;
    if (UE_VERSION >= 501) int32_t NamesReferencedFromExportDataCount;
    if (UE_VERSION >= 501) int64_t PayloadTocOffset;
    if (UE_VERSION >= 502) int32_t DataResourceOffset;
};

struct FGatherableTextData {
    uint64_t Placeholder[4]; // Roughly 32 bytes
};

struct FSHA1 {
    uint64_t Placeholder[3]; // SHA1 hash data
};

type ELinkerType = uint32_t;

/// TEST
struct FLinker : FLinkerTables {
    uint64_t VTable;
    ELinkerType LinkerType;
    UPackage* LinkerRoot;
    FPackageFileSummary Summary;
    TArray<FName> NameMap;
    if (UE_VERSION >= 501) TArray<STUB> SoftObjectPathList;
    if (UE_VERSION >= 409) TArray<FGatherableTextData> GatherableTextDataMap;
    if (UE_VERSION >= 502) TArray<STUB> DataResourceMap;
    if (UE_VERSION < 506) FString Filename;
    bool FilterClientButNotServer;
    bool FilterServerButNotClient;
    FSHA1* ScriptSHA;
};

/// TEST
struct FArchiveUObject : FArchive {
};

struct FStructuredArchive { uint64_t Placeholder; };
struct FBinaryArchiveFormatter { uint64_t Placeholder; };
struct FPackageIndex { int32_t Index; };

/// TEST
struct FEngineVersionBase {
    uint16_t Major;
    uint16_t Minor; 
    uint16_t Patch;
    uint32_t Changelist;
};

/// TEST
struct FEngineVersion : FEngineVersionBase if (UE_VERSION >= 408) {
    if (UE_VERSION >= 408) {
        FString Branch;
    } else {
        uint16_t Major;
        uint16_t Minor; 
        uint16_t Patch;
        uint32_t Changelist;
        FString Branch;
    }
};

struct FFastPathLoadBuffer {
    uint64_t Placeholder[3]; // FArchive::FFastPathLoadBuffer
};

/// TEST
struct FArchiveState {
    uint64_t VTable;

    if (UE_VERSION >= 425) {
        FFastPathLoadBuffer* ActiveFPLB;
        FFastPathLoadBuffer InlineFPLB;
        
        bool ArIsLoading : 1;
        if (UE_VERSION >= 501) bool ArIsLoadingFromCookedPackage : 1;
        bool ArIsSaving : 1;
        bool ArIsTransacting : 1;
        bool ArIsTextFormat : 1;
        bool ArWantBinaryPropertySerialization : 1;
        bool ArUseUnversionedPropertySerialization : 1;
        bool ArForceUnicode : 1;
        bool ArIsPersistent : 1;
        bool ArIsError : 1;
        bool ArIsCriticalError : 1;
        
        if (UE_VERSION >= 500) bool ArShouldSkipCompilingAssets : 1;
        if (UE_VERSION >= 503) bool ArShouldSkipUpdateCustomVersion : 1;
        
        bool ArContainsCode : 1;
        bool ArContainsMap : 1;
        bool ArRequiresLocalizationGather : 1;
        bool ArForceByteSwapping : 1;
        bool ArIgnoreArchetypeRef : 1;
        bool ArNoDelta : 1;
        bool ArNoIntraPropertyDelta : 1;
        bool ArIgnoreOuterRef : 1;
        bool ArIgnoreClassGeneratedByRef : 1;
        bool ArIgnoreClassRef : 1;
        bool ArAllowLazyLoading : 1;
        bool ArIsObjectReferenceCollector : 1;
        bool ArIsModifyingWeakAndStrongReferences : 1;
        bool ArIsCountingMemory : 1;
        bool ArShouldSkipBulkData : 1;
        bool ArIsFilterEditorOnly : 1;
        bool ArIsSaveGame : 1;
        bool ArIsNetArchive : 1;
        bool ArUseCustomPropertyList : 1;
        if (UE_VERSION == 426) bool ArPGI_DontConvertDelegateProperties : 1;
        if (UE_VERSION >= 505) bool ArMergeOverrides : 1;
        if (UE_VERSION >= 506) bool ArPreserveArrayElements : 1;
        
        int32_t ArSerializingDefaults;
        uint32_t ArPortFlags;
        uint64_t ArMaxSerializeSize;
        if (UE_VERSION < 501) {
            int32_t ArUE4Ver;
            int32_t ArLicenseeUE4Ver;
        } else {
            int64_t ArUEVer;
            int32_t ArLicenseeUEVer;
        }
        FEngineVersionBase ArEngineVer;
        if (UE_VERSION < 503) {
            uint32_t ArEngineNetVer;
            uint32_t ArGameNetVer;
        }
        FCustomVersionContainer* CustomVersionContainer;
        void* ArCustomPropertyList; // const FCustomPropertyListNode*

        if (UE_VERSION < 501) void* CookingTargetPlatform; // const ITargetPlatform*
        else if (UE_VERSION < 505) void* CookData; // const UCookOnTheFlyServer*
        else void* SavePackageData; // const UE::SavePackageUtilities::ESavePackageState*

        ZProperty* SerializedProperty;
        void* SerializedPropertyChain; // FArchiveSerializedPropertyChain*
        bool bCustomVersionsAreReset;
        
        if (UE_VERSION >= 426) void* NextProxy;
    } else {
        uint64_t StateData;
    }
};

/// TEST  
struct FArchive : FArchiveState if (UE_VERSION >= 425) {
    if (UE_VERSION >= 425) {
    } else if (UE_VERSION >= 414) {
        uint64_t VTable;

        if (UE_VERSION >= 415) {
            FFastPathLoadBuffer* ActiveFPLB;
            FFastPathLoadBuffer InlineFPLB;
        }
        
        bool ArIsLoading : 1;
        bool ArIsSaving : 1;
        bool ArIsTransacting : 1;
        if (UE_VERSION >= 419) bool ArIsTextFormat : 1;
        bool ArWantBinaryPropertySerialization : 1;
        bool ArForceUnicode : 1;
        bool ArIsPersistent : 1;
        bool ArIsError : 1;
        bool ArIsCriticalError : 1;
        
        bool ArContainsCode : 1;
        bool ArContainsMap : 1;
        bool ArRequiresLocalizationGather : 1;
        bool ArForceByteSwapping : 1;
        bool ArIgnoreArchetypeRef : 1;
        bool ArNoDelta : 1;
        if (UE_VERSION >= 422) bool ArNoIntraPropertyDelta : 1;
        bool ArIgnoreOuterRef : 1;
        bool ArIgnoreClassGeneratedByRef : 1;
        
        bool ArIgnoreClassRef : 1;
        bool ArAllowLazyLoading : 1;
        bool ArIsObjectReferenceCollector : 1;
        bool ArIsModifyingWeakAndStrongReferences : 1;
        bool ArIsCountingMemory : 1;
        bool ArShouldSkipBulkData : 1;
        bool ArIsFilterEditorOnly : 1;
        bool ArIsSaveGame : 1;
        if (UE_VERSION >= 419) bool ArIsNetArchive : 1;
        
        bool ArUseCustomPropertyList : 1;
        
        int32_t ArSerializingDefaults;
        uint32_t ArPortFlags;
        uint64_t ArMaxSerializeSize;
        
        int32_t ArUE4Ver;
        int32_t ArLicenseeUE4Ver;
        FEngineVersionBase ArEngineVer;
        uint32_t ArEngineNetVer;
        uint32_t ArGameNetVer;
        FCustomVersionContainer* CustomVersionContainer;
        void* ArCustomPropertyList; // const FCustomPropertyListNode*
        
        void* CookingTargetPlatform; // const ITargetPlatform*
        ZProperty* SerializedProperty;
        if (UE_VERSION >= 420) void* SerializedPropertyChain; // FArchiveSerializedPropertyChain*
        bool bCustomVersionsAreReset;
    } else {
        uint64_t VTable;

        if (UE_VERSION < 413) int32_t ArNetVer;
        int32_t ArUE4Ver;
        int32_t ArLicenseeUE4Ver;
        
        if (UE_VERSION >= 408) FEngineVersionBase ArEngineVer;
        if (UE_VERSION >= 413) {
            uint32_t ArEngineNetVer;
            uint32_t ArGameNetVer;
        }
        
        FCustomVersionContainer* CustomVersionContainer;
        
        bool ArIsLoading;
        bool ArIsSaving;
        bool ArIsTransacting;
        bool ArWantBinaryPropertySerialization;
        bool ArForceUnicode;
        bool ArIsPersistent;
        bool ArIsError;
        bool ArIsCriticalError;
        bool ArContainsCode;
        bool ArContainsMap;
        bool ArRequiresLocalizationGather;
        bool ArForceByteSwapping;
        bool ArIgnoreArchetypeRef;
        bool ArNoDelta;
        bool ArIgnoreOuterRef;
        if (UE_VERSION >= 410) bool ArIgnoreClassGeneratedByRef;
        bool ArIgnoreClassRef;
        bool ArAllowLazyLoading;
        bool ArIsObjectReferenceCollector;
        bool ArIsModifyingWeakAndStrongReferences;
        bool ArIsCountingMemory;
        bool ArShouldSkipBulkData;
        bool ArIsFilterEditorOnly;
        bool ArIsSaveGame;
        
        if (UE_VERSION == 409) bool ArIsRollback;
        
        int32_t ArSerializingDefaults;
        uint32_t ArPortFlags;
        uint64_t ArMaxSerializeSize;
        
        if (UE_VERSION >= 412) {
            void* ArCustomPropertyList; // const FCustomPropertyListNode*
            bool ArUseCustomPropertyList;
        }
        
        void* CookingTargetPlatform; // const ITargetPlatform*
        ZProperty* SerializedProperty;
        bool bCustomVersionsAreReset;
    }
};

struct FAsyncPackage { uint64_t Placeholder[10]; };

// FLinkerLoad-specific structures
struct FLinkerInstancingContext {
    if (UE_VERSION >= 504) uint64_t Data[2];
    else if (UE_VERSION >= 503) uint64_t Data[39];
    else if (UE_VERSION >= 501) uint64_t Data[41];
    else if (UE_VERSION >= 426) uint64_t Data[10];
    else uint64_t Data[1];
};


template<typename T>
struct TOptional {
    uint64_t Placeholder[2]; // Contains optional value and bool flag
};

/// TEST
class FLinkerLoad : FLinker, FArchiveUObject {
    uint32_t LoadFlags;
    bool bHaveImportsBeenVerified;
    
    if (UE_VERSION >= 411 && UE_VERSION < 504) bool bDynamicClassLinker;
    
    if (UE_VERSION >= 415) {
        UObject* TemplateForGetArchetypeFromLoader;
        bool bForceSimpleIndexToObject;
        bool bLockoutLegacyOperations; 
        if (UE_VERSION == 505) bool bSkipKnownProperties;
        
        if (UE_VERSION >= 423) bool bIsAsyncLoader;
        else bool bLoaderIsFArchiveAsync2;
        
        if (UE_VERSION >= 425) bool bIsDestroyingLoader;
    }
    
    if (UE_VERSION >= 420) {
        FStructuredArchive* StructuredArchive;
        FBinaryArchiveFormatter* StructuredArchiveFormatter;
        TOptional<STUB> StructuredArchiveRootRecord; // TOptional<FStructuredArchive::FRecord>
    }
    
    if (UE_VERSION >= 420 && UE_VERSION < 421) TMap<FName, FPackageIndex> ObjectNameToPackageIndex;
    
    if (UE_VERSION >= 421 && UE_VERSION < 424) {
        TMap<FName, FPackageIndex> ObjectNameToPackageImportIndex;
        TMap<FName, FPackageIndex> ObjectNameToPackageExportIndex;
    }
    
    if (UE_VERSION >= 424) TArray<STUB> ExportReaders;
    if (UE_VERSION >= 500) FPackagePath PackagePath;
    
    if (UE_VERSION < 413) {
        int32_t ExportHash[256];
        FArchive* Loader;
        if (UE_VERSION >= 409) FAsyncPackage* AsyncRoot;
    } else if (UE_VERSION == 424) {
        // UE 4.24: Special order with LocalImportIndices/GlobalImportObjects/ActiveNameMap between Loader and AsyncRoot/ExportHash
        FArchive* Loader;
        int32_t LocalImportIndices;        // PDB shows single int32_t, not TArray
        (UObject*)* GlobalImportObjects;   // PDB shows UObject**, not TArray<UObject*>
        TArray<STUB>* ActiveNameMap;       // PDB shows const TArray<FNameEntryId>*, not TMap
        FAsyncPackage* AsyncRoot;
        TUniquePtr<STUB> ExportHash;       // PDB shows TUniquePtr<int [0]>, not fixed array
    } else {
        FArchive* Loader;
        if (UE_VERSION >= 426) FLinkerInstancingContext InstancingContext;
        if (UE_VERSION >= 501) {
            TUniquePtr<STUB> PackageTrailer; // Package trailer information - TUniquePtr<UE::FPackageTrailer>
            TSet<int32_t> ImportsToVerifyOnCreate;
        }
        if (UE_VERSION >= 409) FAsyncPackage* AsyncRoot;
        if (UE_VERSION >= 425) TUniquePtr<STUB> ExportHash; // UE 4.25+ uses TUniquePtr<int [0]>
        else int32_t ExportHash[256]; // UE 4.13-4.23 uses fixed array
    }
    
    if (UE_VERSION >= 414) TArray<FPackageIndex> PreloadDependencies;
    if (UE_VERSION >= 418) TArray<TFunction> ExternalReadDependencies;
    if (UE_VERSION >= 501) int32_t SoftObjectPathListIndex;
    
    if (UE_VERSION < 423) int32_t NameMapIndex;
    if (UE_VERSION >= 409) int32_t GatherableTextDataMapIndex;
    int32_t ImportMapIndex;
    int32_t ExportMapIndex;
    if (UE_VERSION == 413) int32_t FirstNotLoadedExportMapIndex;
    int32_t DependsMapIndex;
    int32_t ExportHashIndex;
    
    if (UE_VERSION >= 426) {
        bool bHasSerializedPackageFileSummary : 1;
        if (UE_VERSION >= 501) {
            bool bHasSerializedPackageTrailer : 1;
            bool bHasConstructedExportsReaders : 1;
        } else {
            bool bHasReconstructedImportAndExportMap : 1;
        }
        bool bHasSerializedPreloadDependencies : 1;
        bool bHasFixedUpImportMap : 1;
        bool bHasPopulatedInstancingContext : 1;
        if (UE_VERSION >= 501 && UE_VERSION < 503) {
            bool bHasPopulatedRelocationContext : 1;
        } else if (UE_VERSION >= 501) {
            bool bHasRelocatedReferences : 1;
            bool bHasAppliedInstancingContext : 1;
        }
        bool bFixupExportMapDone : 1;
        bool bHasFoundExistingExports : 1;
        bool bHasFinishedInitialization : 1;
        bool bIsGatheringDependencies : 1;
        bool bTimeLimitExceeded : 1;
        bool bUseTimeLimit : 1;
        bool bUseFullTimeLimit : 1;
        if (UE_VERSION >= 501) bool bLoaderNeedsEngineVersionChecks : 1;
    } else {
        bool bHasSerializedPackageFileSummary;
        if (UE_VERSION >= 424) bool bHasReconstructedImportAndExportMap;
        if (UE_VERSION >= 419) bool bHasSerializedPreloadDependencies;
        bool bHasFixedUpImportMap;
        bool bHasFoundExistingExports;
        bool bHasFinishedInitialization;
        bool bIsGatheringDependencies;
        bool bTimeLimitExceeded;
        bool bUseTimeLimit;
        bool bUseFullTimeLimit;
    }
    
    int32_t IsTimeLimitExceededCallCount;
    float TimeLimit;
    double TickStartTime;
    if (UE_VERSION >= 408 && UE_VERSION < 426) bool bFixupExportMapDone;
    int32_t OwnerThread;
    if (UE_VERSION >= 411) bool bForceBlueprintFinalization;
    int32_t DeferredCDOIndex;

    if (UE_VERSION >= 420) TArray<STUB> ResolvingPlaceholderStack;
    else UObject* ResolvingDeferredPlaceholder;

    if (UE_VERSION >= 412) TMap<UObject*, UObject*> ImportPlaceholders;
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

/// TEST
struct FIntPoint {
    int32_t X;
    int32_t Y;
};

/// TEST
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
struct UMetaData {
    uint64_t Placeholder[10];
};

/// TEST
struct FPackageId {
    uint64_t Id;
};

/// TEST
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

/// TEST
struct FPackageFileVersion {
    int32_t FileVersionUE4;
    int32_t FileVersionUE5;
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
    
    if (UE_VERSION >= 417 && UE_VERSION < 427 || UE_VERSION == 500) {
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
        
        if (UE_VERSION >= 412 && UE_VERSION < 420) {
            if (UE_VERSION >= 419) uint64_t ClassUniqueNameIndexMap; // TMap<FName, int> - 8 bytes for UE 4.19+
            else TMap<FName, int> ClassUniqueNameIndexMap;
        }
        if (UE_VERSION >= 414 && UE_VERSION < 419) int32_t PIEInstanceID;
    }
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
    
    if (UE_VERSION >= 501) TFunction PropertyInitCallback;
    if (UE_VERSION >= 506) int32_t SerialNumber;
    if (UE_VERSION >= 500) FObjectInitializerOverrides* SubobjectOverrides;
};
