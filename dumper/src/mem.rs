use crate::{containers::PtrFNamePool, structs::StructInfo};
use anyhow::{Context as _, Result};
use read_process_memory::{CopyAddress as _, ProcessHandle};
use std::{
    collections::HashMap,
    marker::PhantomData,
    mem::MaybeUninit,
    num::NonZero,
    sync::{Arc, Mutex},
};
use ue_reflection::{
    EClassCastFlags, EClassFlags, ECppForm, EEnumFlags, EFunctionFlags, EObjectFlags,
    EPropertyFlags, EStructFlags,
};

pub trait VirtSize<C: StructsTrait> {
    fn size(ctx: &C) -> usize;
}

#[derive(Clone)]
#[repr(C)]
pub struct Ptr<T, C> {
    address: NonZero<usize>,
    ctx: C,
    _type: PhantomData<T>,
}
impl<T, C> std::fmt::Debug for Ptr<T, C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Ptr(0x{:x})", self.address)
    }
}
impl<T, C> Ptr<T, C> {
    pub fn new(address: usize, ctx: C) -> Self {
        Self {
            address: address.try_into().unwrap(),
            ctx,
            _type: Default::default(),
        }
    }
    pub fn new_non_zero(address: NonZero<usize>, ctx: C) -> Self {
        Self {
            address,
            ctx,
            _type: Default::default(),
        }
    }
    pub fn ctx(&self) -> &C {
        &self.ctx
    }
}
impl<T, C: Clone> Ptr<T, C> {
    pub fn map(&self, map: impl FnOnce(usize) -> usize) -> Self {
        Self::new(map(self.address.into()), self.ctx.clone())
    }
    pub fn cast<O>(&self) -> Ptr<O, C> {
        Ptr::new_non_zero(self.address, self.ctx.clone())
    }
    pub fn byte_offset(&self, n: usize) -> Self {
        Self::new_non_zero(self.address.checked_add(n).unwrap(), self.ctx.clone())
    }
}
impl<T: VirtSize<C>, C: Clone + StructsTrait> Ptr<T, C> {
    pub fn offset(&self, n: usize) -> Self {
        self.byte_offset(n * T::size(self.ctx()))
    }
}
impl<T: Pod, C: Mem> Ptr<T, C> {
    pub fn read(&self) -> Result<T> {
        self.ctx.read(self.address.into())
    }
    pub fn read_vec(&self, count: usize) -> Result<Vec<T>> {
        self.ctx.read_vec(self.address.into(), count)
    }
}
impl<T, C: Mem + Clone> Ptr<Option<Ptr<T, C>>, C> {
    pub fn read(&self) -> Result<Option<Ptr<T, C>>> {
        let addr = self.ctx.read::<usize>(self.address.into())?;
        Ok(if addr != 0 {
            Some(self.map(|_| addr).cast())
        } else {
            None
        })
    }
}
impl<T, C: Mem + Clone> Ptr<Ptr<T, C>, C> {
    pub fn read(&self) -> Result<Ptr<T, C>> {
        let addr = self.ctx.read::<usize>(self.address.into())?;
        Ok(self.map(|_| addr).cast())
    }
}

pub trait Pod {}
impl Pod for i8 {}
impl Pod for u8 {}
impl Pod for i16 {}
impl Pod for u16 {}
impl Pod for i32 {}
impl Pod for u32 {}
impl Pod for i64 {}
impl Pod for u64 {}
impl Pod for usize {}
impl Pod for f32 {}
impl Pod for f64 {}
impl Pod for EObjectFlags {}
impl Pod for EClassCastFlags {}
impl Pod for EClassFlags {}
impl Pod for EFunctionFlags {}
impl Pod for EStructFlags {}
impl Pod for EPropertyFlags {}
impl Pod for EEnumFlags {}
impl Pod for ECppForm {}

impl<T: Pod, C: StructsTrait> VirtSize<C> for T {
    fn size(_ctx: &C) -> usize {
        std::mem::size_of::<Self>()
    }
}

impl<T, C: StructsTrait> VirtSize<C> for Ptr<T, C> {
    fn size(_ctx: &C) -> usize {
        8
    }
}
impl<T, C: StructsTrait> VirtSize<C> for Option<Ptr<T, C>> {
    fn size(_ctx: &C) -> usize {
        8
    }
}

pub trait Mem {
    fn read_buf(&self, address: usize, buf: &mut [u8]) -> Result<()>;
    fn read<T>(&self, address: usize) -> Result<T> {
        let mut buf = MaybeUninit::<T>::uninit();
        let bytes = unsafe {
            std::slice::from_raw_parts_mut(
                buf.as_mut_ptr().cast::<u8>() as _,
                std::mem::size_of::<T>(),
            )
        };
        self.read_buf(address, bytes)?;
        Ok(unsafe { std::mem::transmute_copy(&buf) })
    }

    fn read_vec<T: Sized>(&self, address: usize, count: usize) -> Result<Vec<T>> {
        let size = std::mem::size_of::<T>();

        let mut buf = vec![0u8; count * size];
        self.read_buf(address, &mut buf)?;

        let length = buf.len() / size;
        let capacity = buf.capacity() / size;
        let ptr = buf.as_mut_ptr() as *mut T;

        std::mem::forget(buf);

        Ok(unsafe { Vec::from_raw_parts(ptr, length, capacity) })
    }
}
const PAGE_SIZE: usize = 0x1000;
#[derive(Clone)]
pub struct MemCache<M> {
    inner: M,
    pages: Arc<Mutex<HashMap<usize, Vec<u8>>>>,
}
impl<M: Mem> MemCache<M> {
    pub fn wrap(inner: M) -> Self {
        Self {
            inner,
            pages: Default::default(),
        }
    }
}
impl<M: Mem> Mem for MemCache<M> {
    fn read_buf(&self, address: usize, buf: &mut [u8]) -> Result<()> {
        let mut remaining = buf.len();
        let mut cur = 0;

        let mut lock = self.pages.lock().unwrap();

        while remaining > 0 {
            let page_start = (address + cur) & !(PAGE_SIZE - 1);
            let page_offset = (address + cur) - page_start;
            let to_copy = remaining.min(PAGE_SIZE - page_offset);

            let buf_region = &mut buf[cur..cur + to_copy];
            let page_range = page_offset..page_offset + to_copy;
            if let Some(page) = lock.get(&page_start) {
                buf_region.copy_from_slice(&page[page_range]);
            } else {
                let mut page = vec![0; PAGE_SIZE];
                self.inner.read_buf(page_start, &mut page)?;
                buf_region.copy_from_slice(&page[page_range]);
                lock.insert(page_start, page);
            }

            remaining -= to_copy;
            cur += to_copy;
        }

        Ok(())
    }
}

impl Mem for ProcessHandle {
    fn read_buf(&self, address: usize, buf: &mut [u8]) -> Result<()> {
        self.copy_address(address, buf)
            .with_context(|| format!("reading {} bytes at 0x{:x}", buf.len(), address))
    }
}

#[derive(Clone)]
pub struct Ctx<M: Mem> {
    pub mem: M,
    pub fnamepool: PtrFNamePool,
    pub structs: Arc<HashMap<String, StructInfo>>,
    pub version: (u16, u16),
    pub case_preserving: bool,
}
impl<M: Mem> Mem for Ctx<M> {
    fn read_buf(&self, address: usize, buf: &mut [u8]) -> Result<()> {
        self.mem.read_buf(address, buf)
    }
}
impl<M: Mem> NameTrait for Ctx<M> {
    fn fnamepool(&self) -> PtrFNamePool {
        self.fnamepool
    }
}
impl<M: Mem> StructsTrait for Ctx<M> {
    fn get_struct(&self, struct_name: &str) -> &StructInfo {
        let Some(s) = self.structs.get(struct_name) else {
            panic!("struct {struct_name} not found");
        };
        s
    }
    fn struct_member(&self, struct_name: &str, member_name: &str) -> usize {
        let Some(member) = self
            .get_struct(struct_name)
            .members
            .iter()
            .find(|m| m.name == member_name)
        else {
            panic!("struct member {struct_name}::{member_name} not found");
        };
        member.offset as usize
    }
}
impl<M: Mem> VersionTrait for Ctx<M> {
    fn ue_version(&self) -> (u16, u16) {
        self.version
    }
    fn case_preserving(&self) -> bool {
        self.case_preserving
    }
}

pub trait NameTrait {
    fn fnamepool(&self) -> PtrFNamePool;
}
pub trait StructsTrait {
    fn get_struct(&self, struct_name: &str) -> &StructInfo;
    fn struct_member(&self, struct_name: &str, member_name: &str) -> usize;
}
pub trait VersionTrait {
    fn ue_version(&self) -> (u16, u16);
    fn case_preserving(&self) -> bool;
}
