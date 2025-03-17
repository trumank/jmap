use anyhow::Result;
use derive_where::derive_where;

use alloc::*;

use crate::mem::{CtxPtr, ExternalPtr, Mem, NameTrait};

#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct FString(pub TArray<u16>);
impl<C: Mem + Clone> CtxPtr<FString, C> {
    pub fn read(&self) -> Result<String> {
        let array = self.cast::<TArray<u16>>();
        Ok(if let Some(chars) = array.data()? {
            let chars = chars.read_vec(array.len()?)?;
            let len = chars.iter().position(|c| *c == 0).unwrap_or(chars.len());
            String::from_utf16(&chars[..len])?
        } else {
            "".to_string()
        })
    }
}

#[derive_where(Debug, Clone, Copy; T, A::ForElementType<T>)]
#[repr(C)]
pub struct TArray<T, A: TAlloc = TSizedHeapAllocator<32>> {
    pub data: A::ForElementType<T>,
    pub num: u32,
    pub max: u32,
}
impl<C: Mem + Clone, T: Clone, A: TAlloc> CtxPtr<TArray<T, A>, C> {
    pub fn iter(&self) -> Result<impl Iterator<Item = CtxPtr<T, C>> + '_> {
        let data = self.data()?;
        Ok((0..self.len()?).map(move |i| data.as_ref().unwrap().offset(i)))
    }
    //pub fn read(&self, mem: &impl Mem) -> Result<Vec<T>> {
    //    let data = self
    //        .byte_offset(std::mem::offset_of!(TArray<T, A>, data))
    //        .cast::<A::ForElementType<T>>()
    //        .read()?;
    //    Ok(if let Some(data) = <A as TAlloc>::ForElementType::<T>::data(&self.data)? {
    //        data.read_vec(mem, self.num as usize)?
    //    } else {
    //        vec![]
    //    })
    //}
}
impl<C: Mem + Clone, T, A: TAlloc> CtxPtr<TArray<T, A>, C> {
    pub fn data(&self) -> Result<Option<CtxPtr<T, C>>> {
        let alloc = self
            .byte_offset(std::mem::offset_of!(TArray<T, A>, data))
            .cast::<A::ForElementType<T>>();

        <A as TAlloc>::ForElementType::<T>::data(&alloc)
    }
}
impl<C: Mem + Clone, T, A: TAlloc> CtxPtr<TArray<T, A>, C> {
    pub fn len(&self) -> Result<usize> {
        Ok(self
            .byte_offset(std::mem::offset_of!(TArray<T, A>, num))
            .cast::<u32>()
            .read()? as usize)
    }
}

#[derive_where(Debug, Clone, Copy; A::ForElementType<u32>)]
#[repr(C)]
pub struct TBitArray<A: TAlloc> {
    pub allocator_instance: A::ForElementType<u32>,
    pub num_bits: i32,
    pub max_bits: i32,
}

#[derive_where(Debug, Clone, Copy; T, <A::ElementAllocator as TAlloc>::ForElementType<T>, <A::BitArrayAllocator as TAlloc>::ForElementType<u32>)]
#[repr(C)]
pub struct TSparseArray<T, A: TSparseAlloc = FDefaultSparseArrayAllocator> {
    // TArray<TSparseArrayElementOrFreeListLink<TAlignedBytes<32,8> >,TSizedDefaultAllocator<32> >
    pub data: TArray<T, A::ElementAllocator>,
    // TBitArray<FDefaultBitArrayAllocator>
    pub allocation_flags: TBitArray<A::BitArrayAllocator>,
    pub first_free_index: i32,
    pub num_free_indices: i32,
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct TMap<K, V> {
    pub base: TSortableMapBase<K, V>,
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct TSortableMapBase<K, V> {
    pub base: TMapBase<K, V>,
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct TMapBase<K, V> {
    pub pairs: TSet<TTuple<K, V>>,
}
//TSet<TTuple<FName,FString>,TDefaultMapHashableKeyFuncs<FName,FString,0>,FDefaultSetAllocator>

#[derive_where(Debug, Clone, Copy; T,
    <<<A as TSetAlloc>::SparseArrayAllocator as TSparseAlloc>::BitArrayAllocator as TAlloc>::ForElementType<u32>,
    <<<A as TSetAlloc>::SparseArrayAllocator as TSparseAlloc>::ElementAllocator as TAlloc>::ForElementType<TSetElement<T>>,
    <<A as TSetAlloc>::HashAllocator as TAlloc>::ForElementType<FSetElementId>,
)]
#[repr(C)]
pub struct TSet<T, A: TSetAlloc = FDefaultSetAllocator> {
    // TODO hash functions
    pub elements: TSparseArray<TSetElement<T>, <A as TSetAlloc>::SparseArrayAllocator>,
    pub hash: <<A as TSetAlloc>::HashAllocator as TAlloc>::ForElementType<FSetElementId>,
    pub hash_size: i32,
}

const ASDF2: [u8; 0x50] = [0; std::mem::size_of::<TSet<TTuple<FName, ExternalPtr<()>>>>()];

//#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct TSparseArray_TBaseIterator<const N: usize, T, A: TSparseAlloc> {
    pub array: ExternalPtr<TSparseArray<T, A>>,
    pub bit_array_it: TConstSetBitIterator<A::BitArrayAllocator>,
}

pub mod alloc {
    use super::*;
    use crate::mem::{CtxPtr, ExternalPtr, Mem};
    use std::marker::PhantomData;

    pub type FDefaultAllocator = TSizedDefaultAllocator<32>;
    pub type TSizedDefaultAllocator<const P: usize> = TSizedHeapAllocator<P>;
    pub type FDefaultBitArrayAllocator = TInlineAllocator<4, FDefaultAllocator>;
    pub type FDefaultSetAllocator = TSetAllocator;

    pub trait TAlloc {
        type ForElementType<T>: TAllocImpl<T>;
    }
    pub trait TAllocImpl<T> {
        fn data<C: Mem + Clone>(this: &CtxPtr<Self, C>) -> Result<Option<CtxPtr<T, C>>>
        where
            Self: Sized;
    }

    #[derive(Debug, Clone, Copy)]
    pub struct TInlineAllocator<const N: usize, A: TAlloc = FDefaultAllocator>(PhantomData<A>);
    impl<const N: usize, A: TAlloc> TAlloc for TInlineAllocator<N, A> {
        type ForElementType<T> = TInlineAlloc_ForElementType<N, T, A>;
    }
    #[derive(Debug, Clone, Copy)]
    #[repr(C)]
    pub struct TInlineAlloc_ForElementType<const N: usize, T, A: TAlloc> {
        inline_data: [T; N],
        secondary_data: A::ForElementType<T>,
    }
    impl<const N: usize, T, A: TAlloc> TAllocImpl<T> for TInlineAlloc_ForElementType<N, T, A> {
        fn data<C: Mem + Clone>(this: &CtxPtr<Self, C>) -> Result<Option<CtxPtr<T, C>>>
        where
            Self: Sized,
        {
            let second = this
                .byte_offset(std::mem::offset_of!(Self, secondary_data))
                .cast::<A::ForElementType<T>>();
            let a = <A as TAlloc>::ForElementType::<T>::data(&second)?;
            Ok(if let Some(a) = a {
                Some(a)
            } else {
                Some(
                    this.byte_offset(std::mem::offset_of!(Self, inline_data))
                        .cast::<T>(),
                )
            })
        }
    }

    #[derive(Debug, Clone, Copy)]
    pub struct TSizedHeapAllocator<const N: usize>;
    impl<const N: usize> TAlloc for TSizedHeapAllocator<N> {
        type ForElementType<T> = THeapAlloc_ForElementType<N, T>;
    }
    #[derive(Debug, Clone, Copy)]
    #[repr(C)]
    pub struct THeapAlloc_ForElementType<const N: usize, T> {
        data: Option<ExternalPtr<T>>,
    }
    impl<const N: usize, T> TAllocImpl<T> for THeapAlloc_ForElementType<N, T> {
        fn data<C: Mem + Clone>(this: &CtxPtr<Self, C>) -> Result<Option<CtxPtr<T, C>>>
        where
            Self: Sized,
        {
            this.cast::<Option<ExternalPtr<T>>>().read()
        }
    }

    pub trait TSparseAlloc {
        type ElementAllocator: TAlloc;
        type BitArrayAllocator: TAlloc;
    }
    pub struct FDefaultSparseArrayAllocator;
    impl TSparseAlloc for FDefaultSparseArrayAllocator {
        type ElementAllocator = FDefaultAllocator;
        type BitArrayAllocator = FDefaultBitArrayAllocator;
    }

    pub trait TSetAlloc {
        type SparseArrayAllocator: TSparseAlloc;
        type HashAllocator: TAlloc;
        const AverageNumberOfElementsPerHashBucket: usize;
        const BaseNumberOfHashBuckets: usize;
        const MinNumberOfHashedElements: usize;
    }
    pub struct TSetAllocator<
        S = FDefaultSparseArrayAllocator,
        H = TInlineAllocator<1, FDefaultAllocator>,
        const E: usize = 2,
        const B: usize = 8,
        const M: usize = 4,
    >(PhantomData<S>, PhantomData<H>);
    impl<S: TSparseAlloc, H: TAlloc, const E: usize, const B: usize, const M: usize> TSetAlloc
        for TSetAllocator<S, H, E, B, M>
    {
        type SparseArrayAllocator = S;
        type HashAllocator = H;
        const AverageNumberOfElementsPerHashBucket: usize = E;
        const BaseNumberOfHashBuckets: usize = B;
        const MinNumberOfHashedElements: usize = M;
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct FRelativeBitReference {
    pub DWORDIndex: i32,
    pub Mask: u32,
}

#[derive_where(Debug, Clone, Copy; <A as TAlloc>::ForElementType<u32>)]
#[repr(C)]
pub struct TConstSetBitIterator<A: TAlloc> {
    pub bit_reference: FRelativeBitReference,
    pub array: ExternalPtr<TBitArray<A>>,
    pub UnvisitedBitMask: u32,
    pub CurrentBitIndex: i32,
    pub BaseBitIndex: i32,
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct TSetElement<T> {
    pub inner: TSetElementBase<T, 1>,
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct TTuple<K, V> {
    pub a: K,
    pub b: V,
}
impl<C: Clone, K, V> CtxPtr<TTuple<K, V>, C> {
    pub fn a(&self) -> CtxPtr<K, C> {
        self.byte_offset(std::mem::offset_of!(TTuple<K, V>, a))
            .cast::<K>()
    }
    pub fn b(&self) -> CtxPtr<V, C> {
        self.byte_offset(std::mem::offset_of!(TTuple<K, V>, b))
            .cast::<V>()
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct TSetElementBase<T, const N: usize> {
    pub Value: T,
    pub HashNextId: FSetElementId,
    pub HashIndex: i32,
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct FSetElementId {
    pub index: i32,
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct FNameEntryId {
    pub Value: u32,
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct FName {
    pub ComparisonIndex: FNameEntryId,
    pub Number: u32,
}
impl<C: Mem + Clone + NameTrait> CtxPtr<FName, C> {
    pub fn read(&self) -> Result<String> {
        // TODO dynamic struct member
        let value = self.cast::<u32>().read()?;
        let number = self.byte_offset(4).cast::<u32>().read()?;
        let mem = self.ctx();

        let blocks = ExternalPtr::<ExternalPtr<u8>>::new(mem.fnamepool().0 + 0x10);

        let block_index = (value >> 16) as usize;
        let offset = (value & 0xffff) as usize * 2;

        let block = blocks.offset(block_index).read(mem)?;

        let header_bytes: [u8; 2] = block.offset(offset).read_vec(mem, 2)?.try_into().unwrap();
        let header: u16 = unsafe { std::mem::transmute_copy(&header_bytes) };

        // TODO depends on case preserving
        let len = (header >> 6) as usize;
        let is_wide = header & 1 != 0;

        let base = if is_wide {
            String::from_utf16(
                &block
                    .offset(offset + 2)
                    .read_vec(mem, len * 2)?
                    .chunks(2)
                    .map(|chunk| u16::from_le_bytes(chunk.try_into().unwrap()))
                    .collect::<Vec<_>>(),
            )?
        } else {
            String::from_utf8(block.offset(offset + 2).read_vec(mem, len)?)?
        };
        Ok(if number == 0 {
            base
        } else {
            format!("{base}_{}", number - 1)
        })
    }
}

#[derive(Debug, Clone)]
#[repr(C)]
struct FNameBlock {
    data: [u8; 0x1_0000],
}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct FNameEntryAllocator {
    pub Lock: *const (), //FWindowsRWLock Lock;
    pub CurrentBlock: u32,
    pub CurrentByteCursor: u32,
    pub Blocks: [ExternalPtr<[u8; 0x2_0000]>; 0x1_0000],
}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct FNamePool {
    /* offset 0x0000 */
    pub Entries: FNameEntryAllocator,
    /* offset 0x10040 */ // FNamePoolShard<1>[1024] ComparisonShards;
    /* offset 0x10440 */ // FNameEntryId[2808] ENameToEntry;
    /* offset 0x10f38 */ // uint32_t LargestEnameUnstableId;
    /* offset 0x10f40 */ // TMap<FNameEntryId,enum EName,TInlineSetAllocator<512,TSetAllocator<TSparseArrayAllocator<TSizedDefaultAllocator<32>,TSizedDefaultAllocator<32> >,TSizedDefaultAllocator<32>,2,8,4>,2,4>,TDefaultMapHashableKeyFuncs<FNameEntryId,enum EName,0> > EntryToEName;
}

#[derive(Debug, Clone, Copy)]
pub struct PtrFNamePool(pub usize);
