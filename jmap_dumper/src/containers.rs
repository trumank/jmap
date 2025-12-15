use crate::mem::{Ctx, Pod, VirtSize};
use anyhow::Result;
use derive_where::derive_where;
use std::collections::BTreeMap;

use alloc::*;

use crate::mem::{Mem, Ptr};

#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct FString(pub TArray<u16>);
impl<C: Mem> Ptr<FString, C> {
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

#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct FUtf8String(pub TArray<u16>);
impl<C: Mem> Ptr<FUtf8String, C> {
    pub fn read(&self) -> Result<String> {
        let array = self.cast::<TArray<u8>>();
        Ok(if let Some(chars) = array.data()? {
            let chars = chars.read_vec(array.len()?)?;
            String::from_utf8(chars)?
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
impl<C: Ctx, T: Clone + VirtSize<C>, A: TAlloc> Ptr<TArray<T, A>, C> {
    pub fn iter(&self) -> Result<impl Iterator<Item = Ptr<T, C>> + '_> {
        let data = self.data()?;
        Ok((0..self.len()?).map(move |i| data.as_ref().unwrap().offset(i)))
    }
}
impl<C: Mem, T, A: TAlloc> Ptr<TArray<T, A>, C> {
    pub fn data(&self) -> Result<Option<Ptr<T, C>>> {
        let alloc = self
            .byte_offset(std::mem::offset_of!(TArray<T, A>, data))
            .cast::<A::ForElementType<T>>();

        <A as TAlloc>::ForElementType::<T>::data(&alloc)
    }
}
impl<C: Mem, T: Pod, A: TAlloc> Ptr<TArray<T, A>, C> {
    pub fn read_vec(&self) -> Result<Vec<T>> {
        if let Some(data) = self.data()? {
            data.read_vec(self.len()?)
        } else {
            Ok(vec![])
        }
    }
}
impl<C: Mem, T, A: TAlloc> Ptr<TArray<T, A>, C> {
    pub fn len(&self) -> Result<usize> {
        Ok(self
            .byte_offset(std::mem::offset_of!(TArray<T, A>, num))
            .cast::<u32>()
            .read()? as usize)
    }
}

pub mod alloc {
    use super::*;
    use crate::mem::{Mem, Ptr};
    use std::marker::PhantomData;

    pub trait TAlloc {
        type ForElementType<T>: TAllocImpl<T>;
    }
    pub trait TAllocImpl<T> {
        fn data<C: Mem>(this: &Ptr<Self, C>) -> Result<Option<Ptr<T, C>>>
        where
            Self: Sized;
    }

    #[derive(Debug, Clone, Copy)]
    pub struct TSizedHeapAllocator<const N: usize>;
    impl<const N: usize> TAlloc for TSizedHeapAllocator<N> {
        type ForElementType<T> = THeapAlloc_ForElementType<N, T>;
    }
    #[derive(Debug, Clone, Copy)]
    #[repr(C)]
    pub struct THeapAlloc_ForElementType<const N: usize, T> {
        data: usize,
        _phantom: PhantomData<T>,
    }
    impl<const N: usize, T> TAllocImpl<T> for THeapAlloc_ForElementType<N, T> {
        fn data<C: Mem>(this: &Ptr<Self, C>) -> Result<Option<Ptr<T, C>>>
        where
            Self: Sized,
        {
            this.cast::<Option<Ptr<T, _>>>().read()
        }
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct FNameEntryId {}
impl<C: Clone + Ctx> Ptr<FNameEntryId, C> {
    pub fn value(&self) -> Ptr<u32, C> {
        self.byte_offset(0).cast()
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct FName;
impl<C: Clone + Ctx> Ptr<FName, C> {
    pub fn comparison_index(&self) -> Ptr<FNameEntryId, C> {
        let offset = self.ctx().struct_member("FName", "ComparisonIndex");
        self.byte_offset(offset).cast()
    }
    pub fn number(&self) -> Ptr<u32, C> {
        let offset = self.ctx().struct_member("FName", "Number");
        self.byte_offset(offset).cast()
    }
}
impl<C: Ctx> Ptr<FName, C> {
    pub fn read(&self) -> Result<String> {
        let number = self.number().read()?;
        let value = self.comparison_index().value().read()?;
        let mem = self.ctx();

        let case_preserving = mem.case_preserving();

        if mem.ue_version() < (4, 22) {
            // wtf :skull_emoji:
            let chunks = self
                .map(|_| mem.fnamepool().0)?
                .cast::<Ptr<Ptr<Ptr<(), C>, C>, C>>()
                .read()?;

            let per_chunk = 0x4000;

            let chunk = value / per_chunk;
            let offset = value % per_chunk;

            let chunk = chunks.offset(chunk as usize).read()?;
            let entry = chunk.offset(offset as usize).read()?;

            let index = entry.cast::<u32>().read()?;
            let is_wide = (index & 1) == 1;
            let char_data = entry.byte_offset(0x10);

            let base = if is_wide {
                let mut data = vec![];
                let char_data = char_data.cast::<u16>();
                for i in 0.. {
                    let next = char_data.offset(i).read()?;
                    if next == 0 {
                        break;
                    }
                    data.push(next);
                }
                String::from_utf16(&data)?
            } else {
                let mut data = vec![];
                let char_data = char_data.cast::<u8>();
                for i in 0.. {
                    let next = char_data.offset(i).read()?;
                    if next == 0 {
                        break;
                    }
                    data.push(next);
                }
                String::from_utf8(data)?
            };
            return Ok(if number == 0 {
                base
            } else {
                format!("{base}_{}", number - 1)
            });
        }

        let blocks = self.map(|_| mem.fnamepool().0 + 0x10)?.cast::<Ptr<u8, C>>();

        let block_index = (value >> 16) as usize;
        let offset = if case_preserving {
            (value & 0xffff) as usize * 4 + 4
        } else {
            (value & 0xffff) as usize * 2
        };

        let block = blocks.offset(block_index).read()?;
        let header = block.offset(offset).cast::<u16>().read()?;

        let len = if case_preserving {
            (header >> 1) as usize
        } else {
            (header >> 6) as usize
        };
        let is_wide = header & 1 != 0;

        let data = block.offset(offset + 2);
        let base = if is_wide {
            String::from_utf16(
                &data
                    .read_vec(len * 2)?
                    .chunks(2)
                    .map(|chunk| u16::from_le_bytes(chunk.try_into().unwrap()))
                    .collect::<Vec<_>>(),
            )?
        } else {
            String::from_utf8(data.read_vec(len)?)?
        };
        Ok(if number == 0 {
            base
        } else {
            format!("{base}_{}", number - 1)
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PtrFNamePool(pub u64);

pub fn extract_fnames<C: Ctx>(ctx: &C) -> Result<BTreeMap<u32, String>> {
    let mut names = BTreeMap::new();

    let fname_pool_address = ctx.fnamepool().0;
    let ue_version = ctx.ue_version();
    let case_preserving = ctx.case_preserving();

    if ue_version < (4, 22) {
        anyhow::bail!("unimplemented fname extraction for < 4.22")
    } else {
        let block_len = 0x20000;

        let block_count = {
            let mut buf = [0u8; 4];
            ctx.read_buf(fname_pool_address + 8, &mut buf)?;
            u32::from_le_bytes(buf) as usize
        };

        for block_index in 0..=block_count {
            let block_ptr = {
                let mut buf = [0u8; 8];
                ctx.read_buf(
                    fname_pool_address + 0x10 + (block_index as u64) * 8,
                    &mut buf,
                )?;
                u64::from_le_bytes(buf)
            };

            let mut chunk = vec![0u8; block_len];
            if ctx.read_buf(block_ptr, &mut chunk).is_err() {
                continue;
            }

            let mut possible = vec![];
            let mut i = 2;
            let mut bytes_iter = chunk.iter().skip(2);

            // assume FNames cannot contain chars < 32
            while let Some(pos) = bytes_iter.position(|b| !(32..).contains(b)) {
                possible.push((i - 2)..(i + pos + 1));
                i += pos + 1;
            }

            // Validate potential FName entries in printable regions
            for range in &possible {
                let base = range.start;
                let p = &chunk[range.clone()];
                for i in 2..p.len() {
                    let index = base + i - 2;

                    // filter invalid alignment
                    if case_preserving {
                        if index % 4 != 0 {
                            continue;
                        }
                    } else if index % 2 != 0 {
                        continue;
                    }

                    let header = ((p[i - 1] as u16) << 8) + p[i - 2] as u16;

                    let len = if case_preserving {
                        (header >> 1) as usize
                    } else {
                        (header >> 6) as usize
                    };
                    let is_wide = (header & 1) != 0;

                    if len > 0
                        && !is_wide
                        && i + len < p.len()
                        && let Ok(name) = String::from_utf8(p[i..(i + len)].to_vec())
                    {
                        let value = if case_preserving {
                            ((block_index << 16) | (index / 4)) as u32
                        } else {
                            ((block_index << 16) | (index / 2)) as u32
                        };
                        names.insert(value, name);
                    }
                }
            }
        }
    }

    Ok(names)
}

// FScriptArray - untyped array for runtime element access
#[derive(Debug, Clone, Copy)]
pub struct FScriptArray;
impl<C: Ctx> Ptr<FScriptArray, C> {
    pub fn data(&self) -> Ptr<Option<Ptr<u8, C>>, C> {
        let offset = self.ctx().struct_member("FScriptArray", "Data");
        self.byte_offset(offset).cast()
    }
    pub fn num(&self) -> Ptr<i32, C> {
        let offset = self.ctx().struct_member("FScriptArray", "ArrayNum");
        self.byte_offset(offset).cast()
    }
}

// FScriptBitArray - for reading allocation flags in sparse arrays
// Uses FDefaultBitArrayAllocator which has 4 inline DWORDs (128 bits) + overflow pointer
#[derive(Debug, Clone, Copy)]
pub struct FScriptBitArray;

impl<C: Ctx> Ptr<FScriptBitArray, C> {
    /// Get pointer to inline data (first 4 DWORDs)
    pub fn inline_data(&self) -> Ptr<u32, C> {
        let alloc_offset = self
            .ctx()
            .struct_member("FScriptBitArray", "AllocatorInstance");
        let inline_offset = self
            .ctx()
            .struct_member("FDefaultBitArrayAllocator", "InlineData");
        self.byte_offset(alloc_offset + inline_offset).cast()
    }

    /// Get pointer to secondary (heap) data for overflow
    pub fn secondary_data(&self) -> Ptr<Option<Ptr<u32, C>>, C> {
        let alloc_offset = self
            .ctx()
            .struct_member("FScriptBitArray", "AllocatorInstance");
        let secondary_offset = self
            .ctx()
            .struct_member("FDefaultBitArrayAllocator", "SecondaryData");
        self.byte_offset(alloc_offset + secondary_offset).cast()
    }

    pub fn num_bits(&self) -> Ptr<i32, C> {
        let offset = self.ctx().struct_member("FScriptBitArray", "NumBits");
        self.byte_offset(offset).cast()
    }

    /// Check if a specific index is allocated (bit is set)
    pub fn is_allocated(&self, index: usize) -> Result<bool> {
        let num_bits = self.num_bits().read()?;
        if num_bits <= 0 || index >= num_bits as usize {
            return Ok(false);
        }

        let word_index = index / 32;
        let bit_index = index % 32;

        // If secondary pointer is set, ALL data is on heap
        // Otherwise, ALL data is inline
        let word = if let Some(secondary_ptr) = self.secondary_data().read()? {
            secondary_ptr.offset(word_index).read()?
        } else {
            self.inline_data().offset(word_index).read()?
        };

        Ok((word & (1 << bit_index)) != 0)
    }
}

// FScriptSparseArray - for iterating valid entries
#[derive(Debug, Clone, Copy)]
pub struct FScriptSparseArray;
impl<C: Ctx> Ptr<FScriptSparseArray, C> {
    pub fn data(&self) -> Ptr<FScriptArray, C> {
        let offset = self.ctx().struct_member("FScriptSparseArray", "Data");
        self.byte_offset(offset).cast()
    }
    pub fn allocation_flags(&self) -> Ptr<FScriptBitArray, C> {
        let offset = self
            .ctx()
            .struct_member("FScriptSparseArray", "AllocationFlags");
        self.byte_offset(offset).cast()
    }
    /// Get the maximum index (Data.ArrayNum)
    pub fn get_max_index(&self) -> Result<usize> {
        Ok(self.data().num().read()? as usize)
    }
    /// Check if an index is valid (allocated, not free)
    pub fn is_valid_index(&self, index: usize) -> Result<bool> {
        self.allocation_flags().is_allocated(index)
    }
    /// Get pointer to element data at index (caller must know element size)
    pub fn get_data(&self, index: usize, element_size: usize) -> Result<Ptr<u8, C>> {
        let data_ptr = self.data().data().read()?.expect("sparse array data");
        Ok(data_ptr.byte_offset(index * element_size))
    }
}

// FScriptSet - for iterating set elements
#[derive(Debug, Clone, Copy)]
pub struct FScriptSet;
impl<C: Ctx> Ptr<FScriptSet, C> {
    pub fn elements(&self) -> Ptr<FScriptSparseArray, C> {
        let offset = self.ctx().struct_member("FScriptSet", "Elements");
        self.byte_offset(offset).cast()
    }
}

// FScriptMap - for iterating map pairs (same layout as Set, stores pairs)
#[derive(Debug, Clone, Copy)]
pub struct FScriptMap;
impl<C: Ctx> Ptr<FScriptMap, C> {
    pub fn pairs(&self) -> Ptr<FScriptSet, C> {
        let offset = self.ctx().struct_member("FScriptMap", "Pairs");
        self.byte_offset(offset).cast()
    }
}
