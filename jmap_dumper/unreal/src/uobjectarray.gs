import unreal::core::{UE_VERSION, UE_PACK_FUOBJECT_ITEM, int8_t, uint8_t, int32_t, uint32_t, int64_t, uint64_t};
import unreal::containers::{TArray};
import unreal::objects::{UObject};
import unreal::unreal::{STUB, FCriticalSection};

/// TEST
struct FThreadSafeCounter {
    uint32_t Counter;
};

/// TEST
struct member_pack(if (UE_PACK_FUOBJECT_ITEM) 4 else 8) FUObjectItem {
    if (UE_VERSION >= 507) int64_t FlagsAndRefCount;
    UObject* Object;

    if (UE_VERSION < 413) int32_t ClusterAndFlags;
    if (UE_VERSION >= 413 && UE_VERSION < 507) int32_t Flags;
    if (UE_VERSION >= 413 && UE_VERSION < 416) int32_t ClusterIndex;
    if (UE_VERSION >= 416 && UE_VERSION < 507) int32_t ClusterRootIndex;

    int32_t SerialNumber;
	if (UE_VERSION >= 507) int32_t ClusterRootIndex;
    if (UE_VERSION >= 505 && UE_VERSION < 507) int32_t RefCount;
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
    if (UE_VERSION >= 508) {
		int32_t NumElements;
		int32_t MaxElements;
		int32_t NumChunks;
		int32_t MaxChunks;
		FUObjectItem* PreAllocatedObjects;
	} else {
		FUObjectItem* PreAllocatedObjects;
		int32_t MaxElements;
		int32_t NumElements;
		int32_t MaxChunks;
		int32_t NumChunks;
	}
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
	if (UE_VERSION >= 508) FChunkedFixedUObjectArray ObjObjects;
    
    int32_t ObjFirstGCIndex;
    int32_t ObjLastNonGCIndex;
    if (UE_VERSION >= 411) int32_t MaxObjectsNotConsideredByGC;
    if (UE_VERSION < 411) int32_t OpenForDisregardForGC;
    else bool OpenForDisregardForGC;

    if          (UE_VERSION == 407) FUObjectArrayOlder ObjObjects;
    else if     (UE_VERSION <  411) FUObjectArrayOld ObjObjects;
    else if     (UE_VERSION <  420) FFixedUObjectArray ObjObjects;
    else if     (UE_VERSION <  508) FChunkedFixedUObjectArray ObjObjects;

    if          (UE_VERSION == 407) TArray<int> ObjAvailable;
    else {
        if      (UE_VERSION >= 408) FCriticalSection ObjObjectsCritical;
        if      (UE_VERSION <  422) TLockFreePointerListUnordered<int, 128> ObjAvailableList;
        else if (UE_VERSION <  427) TLockFreePointerListUnordered<int, 64> ObjAvailableList;
        else                        TArray<int> ObjAvailableList;
		
		if (UE_VERSION >= 507) int32_t ObjAvailableListEstimateCount;
    }

    TArray<FUObjectCreateListener*> UObjectCreateListeners;
	if (UE_VERSION >= 508) FCriticalSection UObjectCreateListenersCritical;
	
    TArray<FUObjectDeleteListener*> UObjectDeleteListeners;

    if (UE_VERSION >= 409) FCriticalSection UObjectDeleteListenersCritical;

    if (UE_VERSION >= 411) {
        if (UE_VERSION >= 501) FThreadSafeCounter PrimarySerialNumber;
        else FThreadSafeCounter MasterSerialNumber;
    }

    if (UE_VERSION >= 502) bool bShouldRecycleObjectIndices;
};
