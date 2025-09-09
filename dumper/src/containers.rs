use crate::mem::VirtSize;
use anyhow::Result;
use derive_where::derive_where;

use alloc::*;

use crate::mem::{Mem, NameTrait, Ptr, StructsTrait, VersionTrait};

#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct FString(pub TArray<u16>);
impl<C: Mem + Clone> Ptr<FString, C> {
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
impl<C: Mem + Clone + StructsTrait, T: Clone + VirtSize<C>, A: TAlloc> Ptr<TArray<T, A>, C> {
    pub fn iter(&self) -> Result<impl Iterator<Item = Ptr<T, C>> + '_> {
        let data = self.data()?;
        Ok((0..self.len()?).map(move |i| data.as_ref().unwrap().offset(i)))
    }
}
impl<C: Mem + Clone, T, A: TAlloc> Ptr<TArray<T, A>, C> {
    pub fn data(&self) -> Result<Option<Ptr<T, C>>> {
        let alloc = self
            .byte_offset(std::mem::offset_of!(TArray<T, A>, data))
            .cast::<A::ForElementType<T>>();

        <A as TAlloc>::ForElementType::<T>::data(&alloc)
    }
}
impl<C: Mem + Clone, T, A: TAlloc> Ptr<TArray<T, A>, C> {
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
        fn data<C: Mem + Clone>(this: &Ptr<Self, C>) -> Result<Option<Ptr<T, C>>>
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
        fn data<C: Mem + Clone>(this: &Ptr<Self, C>) -> Result<Option<Ptr<T, C>>>
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
impl<C: Clone + StructsTrait> Ptr<FNameEntryId, C> {
    pub fn value(&self) -> Ptr<u32, C> {
        self.byte_offset(0).cast()
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct FName;
impl<C: Clone + StructsTrait> Ptr<FName, C> {
    pub fn comparison_index(&self) -> Ptr<FNameEntryId, C> {
        let offset = self.ctx().struct_member("FName", "ComparisonIndex");
        self.byte_offset(offset).cast()
    }
    pub fn number(&self) -> Ptr<u32, C> {
        let offset = self.ctx().struct_member("FName", "Number");
        self.byte_offset(offset).cast()
    }
}
impl<C: Mem + Clone + NameTrait + StructsTrait + VersionTrait> Ptr<FName, C> {
    pub fn read(&self) -> Result<String> {
        let number = self.number().read()?;
        let value = self.comparison_index().value().read()?;
        let mem = self.ctx();

        let case_preserving = mem.case_preserving();

        if mem.ue_version() < (4, 22) {
            // wtf :skull_emoji:
            let chunks = self
                .map(|_| mem.fnamepool().0 as u64)
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

        let blocks = self
            .map(|_| mem.fnamepool().0 as u64 + 0x10)
            .cast::<Ptr<u8, C>>();

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

        let base = if is_wide {
            String::from_utf16(
                &block
                    .offset(offset + 2)
                    .read_vec(len * 2)?
                    .chunks(2)
                    .map(|chunk| u16::from_le_bytes(chunk.try_into().unwrap()))
                    .collect::<Vec<_>>(),
            )?
        } else {
            String::from_utf8(block.offset(offset + 2).read_vec(len)?)?
        };
        Ok(if number == 0 {
            base
        } else {
            format!("{base}_{}", number - 1)
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PtrFNamePool(pub usize);
