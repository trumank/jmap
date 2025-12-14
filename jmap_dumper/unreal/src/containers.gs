// Unreal Engine Container Types

import unreal::core::int32_t;
import unreal::core::uint64_t;

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

template<typename KeyType, typename ValueType>
public struct TMap {
    uint64_t Data[10];
};

template<typename ElementType>
public struct TSet {
    uint64_t Data[10];
};
