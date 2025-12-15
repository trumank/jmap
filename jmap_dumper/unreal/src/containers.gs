// Unreal Engine Container Types

import unreal::core::{UE_VERSION, int32_t, uint32_t, uint64_t, uint8_t};

public struct FScriptElement {};

// ============================================================================
// Base Allocator Types
// ============================================================================

// Heap allocator - just a pointer to data
template<typename InIndexType>
public struct TSizedHeapAllocator {
    type IndexType = InIndexType;
    FScriptElement* Data;
};

// ============================================================================
// Inline Allocator Building Blocks
// ============================================================================

// TInlineAllocator<N>::ForElementType<uint32> - for bit arrays
template<int NumDWORDs>
public struct TInlineBitArrayAllocatorN {
    uint32_t InlineData[NumDWORDs];
    FScriptElement* SecondaryData;
};

// TInlineAllocator<N>::ForElementType<FSetElementId> - for hash buckets
template<int NumBuckets>
public struct TInlineHashAllocatorN {
    int32_t InlineData[NumBuckets];  // FSetElementId is int32_t
    FScriptElement* SecondaryData;
};

// TInlineAllocator<N>::ForElementType<ElementType> - for element storage
template<int NumElements, typename ElementType>
public struct TInlineElementAllocatorN {
    type IndexType = int32_t;
    uint8_t InlineData[NumElements * sizeof(ElementType)];
    FScriptElement* SecondaryData;
};

// ============================================================================
// TBitArray Allocators
// ============================================================================

// Default: TInlineAllocator<4>::ForElementType<uint32> (128 bits inline)
public struct FDefaultBitArrayAllocator {
    uint32_t InlineData[4];
    FScriptElement* SecondaryData;
};

// ============================================================================
// TBitArray - Dynamic array of bits
// ============================================================================

/// TEST
template<typename InAllocator = FDefaultBitArrayAllocator>
public struct TBitArray {
    InAllocator AllocatorInstance;
    int32_t NumBits;
    int32_t MaxBits;
};

// ============================================================================
// TArray - Dynamic array
// ============================================================================

template<typename InElementType, typename InAllocator = TSizedHeapAllocator<int32_t>>
public struct TArray {
    type ElementType = InElementType;
    type IndexType = InAllocator::typename IndexType;

    InAllocator AllocatorInstance;
    InAllocator::typename IndexType ArrayNum;
    InAllocator::typename IndexType ArrayMax;
};

// ============================================================================
// TSparseArray - Sparse array with stable indices
// ============================================================================

// Union element: either holds data or free list links
template<typename ElementType>
public struct TSparseArrayElementOrFreeListLink {
    ElementType ElementData;
};

// Default sparse array allocator - heap elements + 4 inline DWORDs for bits
public struct FDefaultSparseArrayAllocator {
    type ElementAllocator = TSizedHeapAllocator<int32_t>;
    type BitArrayAllocator = FDefaultBitArrayAllocator;
};

// Inline sparse array allocator
template<int NumElements, typename ElementType>
public struct TInlineSparseArrayAllocator {
    type ElementAllocator = TInlineElementAllocatorN<NumElements, ElementType>;
    type BitArrayAllocator = TInlineBitArrayAllocatorN<(NumElements + 31) / 32>;
};

/// TEST
template<typename InElementType, typename InAllocator = FDefaultSparseArrayAllocator>
public struct TSparseArray {
    TArray<TSparseArrayElementOrFreeListLink<InElementType>, InAllocator::typename ElementAllocator> Data;
    TBitArray<InAllocator::typename BitArrayAllocator> AllocationFlags;
    int32_t FirstFreeIndex;
    int32_t NumFreeIndices;
};

// ============================================================================
// TSet - Hash set container
// ============================================================================

// Set element wrapper: stores value + hash chain info
template<typename InElementType>
public struct TSetElement {
    InElementType Value;
    int32_t HashNextId;   // FSetElementId
    int32_t HashIndex;
};

// Default set allocator: heap sparse array + 1 inline hash bucket
public struct FDefaultSetAllocator {
    type SparseArrayAllocator = FDefaultSparseArrayAllocator;
    type HashAllocator = TInlineHashAllocatorN<1>;
};

// Inline set allocator - matches UE's TInlineSetAllocator<N>
// ElementType = TSetElement<TTuple<K,V>> for TMap, or TSetElement<T> for TSet
template<int NumElements, typename ElementType>
public struct TInlineSetAllocator {
    type SparseArrayAllocator = TInlineSparseArrayAllocator<NumElements, TSparseArrayElementOrFreeListLink<ElementType>>;
    type HashAllocator = TInlineHashAllocatorN<(NumElements + 1) / 2>;
};

/// TEST
template<typename InElementType, typename InAllocator = FDefaultSetAllocator>
public struct TSet {
    TSparseArray<TSetElement<InElementType>, InAllocator::typename SparseArrayAllocator> Elements;
    InAllocator::typename HashAllocator Hash;
    int32_t HashSize;
};

// ============================================================================
// TMap - Hash map container
// ============================================================================

template<typename InKeyType, typename InValueType>
public struct TTuple {
    InKeyType Key;
    InValueType Value;
};

/// TEST
template<typename InKeyType, typename InValueType, typename InAllocator = FDefaultSetAllocator>
public struct TMap {
    type PairType = TTuple<InKeyType, InValueType>;
    TSet<PairType, InAllocator> Pairs;
};

// ============================================================================
// FScript* Types - Untyped runtime access to containers
// These must match the memory layout of the typed versions above
// ============================================================================

// Untyped script array - heap allocated data
public struct FScriptArray {
    void* Data;
    int32_t ArrayNum;
    int32_t ArrayMax;
};

// Untyped bit array - uses FDefaultBitArrayAllocator (4 inline DWORDs)
public struct FScriptBitArray {
    FDefaultBitArrayAllocator AllocatorInstance;
    int32_t NumBits;
    int32_t MaxBits;
};

// Untyped sparse array
public struct FScriptSparseArray {
    FScriptArray Data;
    FScriptBitArray AllocationFlags;
    int32_t FirstFreeIndex;
    int32_t NumFreeIndices;
};

// Untyped set - uses FDefaultSetAllocator (1 inline hash bucket)
public struct FScriptSet {
    FScriptSparseArray Elements;
    TInlineHashAllocatorN<1> Hash;
    int32_t HashSize;
};

// Untyped map (same layout as set, stores key-value pairs)
public struct FScriptMap {
    FScriptSet Pairs;
};
