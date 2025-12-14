import unreal::core::{UE_VERSION, int8_t, uint8_t, int16_t, uint16_t, int32_t, uint32_t, int64_t, uint64_t};
import unreal::containers::{TArray, TMap, TSet};
import unreal::objects::{UObject, UClass};
import unreal::properties::{ZProperty};
import unreal::archive::{FEngineVersion, FArchive, FArchiveUObject, FStructuredArchive, FBinaryArchiveFormatter, FAsyncPackage};
import unreal::unreal::{
    FName, FString, FGuid, FIoHash, FCustomVersionContainer, STUB,
    TUniquePtr, TScopedPointer, FObjectThumbnail, FWorldTileInfo, TFunction,
    UMetaData
};

/// TEST
struct FPackageIndex { int32_t Index; };

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
struct FPackageFileVersion {
    int32_t FileVersionUE4;
    int32_t FileVersionUE5;
};

/// TEST
struct FPackageFileSummary {
    uint32_t Tag;
    if (UE_VERSION < 500) {
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
    if (UE_VERSION >= 500) int32_t NamesReferencedFromExportDataCount;
    if (UE_VERSION >= 500) int64_t PayloadTocOffset;
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
        if (UE_VERSION >= 500) {
            TUniquePtr<STUB> PackageTrailer; // Package trailer information - TUniquePtr<UE::FPackageTrailer>
            if (UE_VERSION >= 501) TSet<int32_t> ImportsToVerifyOnCreate;
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
        if (UE_VERSION >= 500) bool bHasSerializedPackageTrailer : 1;
        if (UE_VERSION >= 501) bool bHasConstructedExportsReaders : 1;
        else bool bHasReconstructedImportAndExportMap : 1;
        bool bHasSerializedPreloadDependencies : 1;
        bool bHasFixedUpImportMap : 1;
        bool bHasPopulatedInstancingContext : 1;
        if (UE_VERSION >= 501 && UE_VERSION < 503) {
            bool bHasPopulatedRelocationContext : 1;
        } else if (UE_VERSION >= 503) {
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
        if (UE_VERSION >= 500) bool bLoaderNeedsEngineVersionChecks : 1;
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
struct FPackageId {
    uint64_t Id;
};

/// TEST
struct FPackagePath {
    if (UE_VERSION >= 500) {
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
    } else if (UE_VERSION >= 500) {
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
