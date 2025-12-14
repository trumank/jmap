import unreal::core::{UE_VERSION, int8_t, uint8_t, int16_t, uint16_t, int32_t, uint32_t, int64_t, uint64_t};
import unreal::properties::{ZProperty};
import unreal::unreal::{FCustomVersionContainer, FString};

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
        if (UE_VERSION >= 500) bool ArIsLoadingFromCookedPackage : 1;
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
        if (UE_VERSION < 500) {
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

struct FArchiveUObject : FArchive {
};

struct FStructuredArchive { uint64_t Placeholder; };
struct FBinaryArchiveFormatter { uint64_t Placeholder; };

struct FAsyncPackage { uint64_t Placeholder[10]; };
