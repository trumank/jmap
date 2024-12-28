use crate::containers::PtrFNamePool;
use anyhow::{Context as _, Result};
use read_process_memory::{CopyAddress as _, ProcessHandle};
use std::{
    collections::HashMap,
    marker::PhantomData,
    mem::MaybeUninit,
    sync::{Arc, Mutex},
};

#[repr(C)]
pub struct ExternalPtr<T> {
    address: usize,
    _type: PhantomData<T>,
}
impl<T> Copy for ExternalPtr<T> {}
impl<T> Clone for ExternalPtr<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T> std::fmt::Debug for ExternalPtr<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ExternalPtr(0x{:x})", self.address)
    }
}
impl<T> ExternalPtr<T> {
    pub fn new(address: usize) -> Self {
        Self {
            address,
            _type: Default::default(),
        }
    }
    pub fn is_null(self) -> bool {
        self.address == 0
    }
    pub fn cast<O>(self) -> ExternalPtr<O> {
        ExternalPtr::new(self.address)
    }
    pub fn byte_offset(&self, n: usize) -> Self {
        Self::new(self.address + n)
    }
    pub fn offset(&self, n: usize) -> Self {
        self.byte_offset(n * std::mem::size_of::<T>())
    }
    pub fn read(&self, mem: &impl Mem) -> Result<T> {
        mem.read(self.address)
    }
    pub fn read_opt(&self, mem: &impl Mem) -> Result<Option<T>> {
        Ok(match self.address {
            0 => None,
            a => Some(mem.read::<T>(a)?),
        })
    }
    pub fn read_vec(&self, mem: &impl Mem, count: usize) -> Result<Vec<T>> {
        mem.read_vec(self.address, count)
    }
    pub fn ctx<C>(self, ctx: C) -> CtxPtr<T, C> {
        CtxPtr {
            address: self.address,
            ctx,
            _type: Default::default(),
        }
    }
}

#[derive(Clone)]
#[repr(C)]
pub struct CtxPtr<T, C> {
    address: usize,
    ctx: C,
    _type: PhantomData<T>,
}
impl<T, C> std::fmt::Debug for CtxPtr<T, C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CtxPtr(0x{:x})", self.address)
    }
}
impl<T, C> CtxPtr<T, C> {
    pub fn new(address: usize, ctx: C) -> Self {
        Self {
            address,
            ctx,
            _type: Default::default(),
        }
    }
    pub fn is_null(&self) -> bool {
        self.address == 0
    }
}
impl<T, C: Clone> CtxPtr<T, C> {
    pub fn cast<O>(&self) -> CtxPtr<O, C> {
        CtxPtr::new(self.address, self.ctx.clone())
    }
    pub fn byte_offset(&self, n: usize) -> Self {
        Self::new(self.address + n, self.ctx.clone())
    }
    pub fn offset(&self, n: usize) -> Self {
        self.byte_offset(n * std::mem::size_of::<T>())
    }
}
impl<T, C: Mem> CtxPtr<T, C> {
    pub fn read(&self) -> Result<T> {
        self.ctx.read(self.address)
    }
    pub fn read_opt(&self) -> Result<Option<T>> {
        Ok(match self.address {
            0 => None,
            a => Some(self.ctx.read::<T>(a)?),
        })
    }
    pub fn read_vec(&self, count: usize) -> Result<Vec<T>> {
        self.ctx.read_vec(self.address, count)
    }
}

#[derive(Debug)]
pub enum FlaggedPtr<T> {
    Local(*const T),
    Remote(ExternalPtr<T>),
}
impl<T> Copy for FlaggedPtr<T> {}
impl<T> Clone for FlaggedPtr<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T> FlaggedPtr<T> {
    pub fn is_null(self) -> bool {
        match self {
            FlaggedPtr::Local(ptr) => ptr.is_null(),
            FlaggedPtr::Remote(ptr) => ptr.is_null(),
        }
    }
}
impl<T: Clone> FlaggedPtr<T> {
    pub fn read(self, mem: &impl Mem) -> Result<T> {
        Ok(match self {
            FlaggedPtr::Local(ptr) => unsafe { ptr.read() },
            FlaggedPtr::Remote(ptr) => ptr.read(mem)?,
        })
    }
    pub fn read_vec(self, mem: &impl Mem, count: usize) -> Result<Vec<T>> {
        Ok(if self.is_null() {
            vec![]
        } else {
            match self {
                FlaggedPtr::Local(ptr) => unsafe {
                    std::slice::from_raw_parts(ptr, count).to_vec()
                },
                FlaggedPtr::Remote(ptr) => ptr.read_vec(mem, count)?,
            }
        })
    }
}

pub trait Mem {
    fn read_buf(&self, address: usize, buf: &mut [u8]) -> Result<()>;
    fn read<T>(&self, address: usize) -> Result<T> {
        let mut buf = MaybeUninit::<T>::uninit();
        let mut bytes = unsafe {
            std::slice::from_raw_parts_mut(
                buf.as_mut_ptr().cast::<u8>() as _,
                std::mem::size_of::<T>(),
            )
        };
        self.read_buf(address, &mut bytes)?;
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
}
impl<M: Mem> Mem for Ctx<M> {
    fn read_buf(&self, address: usize, buf: &mut [u8]) -> Result<()> {
        self.mem.read_buf(address, buf)
    }
}
