use crate::structs::StructInfo;
use anyhow::{Context as _, Result};
use jmap::{
    EClassCastFlags, EClassFlags, ECppForm, EEnumFlags, EFunctionFlags, EObjectFlags,
    EPropertyFlags, EStructFlags,
};
use std::{
    collections::HashMap,
    marker::PhantomData,
    num::NonZero,
    sync::{Arc, Mutex},
};

// --- Pod trait (merged TryFromBytes + Pod) ---

pub trait Pod: Sized {
    fn try_from_bytes(bytes: &[u8]) -> Result<Self>;
}

macro_rules! impl_pod {
    ($($t:ty),* $(,)?) => {
        $(
            impl Pod for $t {
                fn try_from_bytes(bytes: &[u8]) -> Result<Self> {
                    Ok(bytemuck::pod_read_unaligned(bytes))
                }
            }
        )*
    };
}

macro_rules! impl_pod_bitflags {
    ($(($t:ty, $bits_ty:ty)),* $(,)?) => {
        $(
            impl Pod for $t {
                fn try_from_bytes(bytes: &[u8]) -> Result<Self> {
                    let bits: $bits_ty = bytemuck::pod_read_unaligned(bytes);
                    Self::from_bits(bits)
                        .ok_or_else(|| anyhow::anyhow!("Invalid {} bits: 0x{:x}", stringify!($t), bits))
                }
            }
        )*
    };
}

impl_pod!(i8, u8, i16, u16, i32, u32, i64, u64, usize, f32, f64);

impl_pod_bitflags!(
    (EObjectFlags, u32),
    (EClassCastFlags, u64),
    (EClassFlags, u32),
    (EFunctionFlags, u32),
    (EStructFlags, u32),
    (EPropertyFlags, u64),
    (EEnumFlags, u8),
);

impl Pod for ECppForm {
    fn try_from_bytes(bytes: &[u8]) -> Result<Self> {
        let discriminant: u8 = bytemuck::pod_read_unaligned(bytes);
        Self::from_repr(discriminant)
            .ok_or_else(|| anyhow::anyhow!("Invalid ECppForm discriminant: {}", discriminant))
    }
}

// --- Mem trait ---

pub trait Mem: Send + Sync {
    fn read_buf(&self, address: u64, buf: &mut [u8]) -> Result<()>;
    fn write_buf(&self, address: u64, buf: &[u8]) -> Result<()> {
        let _ = (address, buf);
        anyhow::bail!("write not supported for this memory backend")
    }
    fn clear_cache(&self) {}
}

// --- MemCache ---

const PAGE_SIZE: usize = 0x1000;
pub struct MemCache<M> {
    inner: M,
    pages: Mutex<HashMap<u64, Vec<u8>>>,
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
    fn write_buf(&self, address: u64, buf: &[u8]) -> Result<()> {
        // Invalidate any cached pages that overlap the write
        let mut lock = self.pages.lock().unwrap();
        let start_page = address & !(PAGE_SIZE as u64 - 1);
        let end_page = (address + buf.len() as u64).saturating_sub(1) & !(PAGE_SIZE as u64 - 1);
        let mut page = start_page;
        while page <= end_page {
            lock.remove(&page);
            page += PAGE_SIZE as u64;
        }
        drop(lock);
        self.inner.write_buf(address, buf)
    }

    fn read_buf(&self, address: u64, buf: &mut [u8]) -> Result<()> {
        let mut remaining = buf.len();
        let mut cur = 0;

        let mut lock = self.pages.lock().unwrap();

        while remaining > 0 {
            let page_start = (address + cur as u64) & !(PAGE_SIZE as u64 - 1);
            let page_offset = address as usize + cur - page_start as usize;
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

    fn clear_cache(&self) {
        self.pages.lock().unwrap().clear();
    }
}

// --- ProcessHandle ---

pub struct ProcessHandle {
    pub pid: i32,
}

impl ProcessHandle {
    pub fn new(pid: i32) -> Self {
        Self { pid }
    }
}

#[cfg(target_os = "linux")]
impl Mem for ProcessHandle {
    fn read_buf(&self, address: u64, buf: &mut [u8]) -> Result<()> {
        let local_iov = libc::iovec {
            iov_base: buf.as_mut_ptr() as *mut libc::c_void,
            iov_len: buf.len(),
        };
        let remote_iov = libc::iovec {
            iov_base: address as *mut libc::c_void,
            iov_len: buf.len(),
        };
        let result = unsafe { libc::process_vm_readv(self.pid, &local_iov, 1, &remote_iov, 1, 0) };
        if result == -1 {
            anyhow::bail!(
                "process_vm_readv failed reading {} bytes at 0x{:x}: {}",
                buf.len(),
                address,
                std::io::Error::last_os_error()
            );
        }
        Ok(())
    }

    fn write_buf(&self, address: u64, buf: &[u8]) -> Result<()> {
        let local_iov = libc::iovec {
            iov_base: buf.as_ptr() as *mut libc::c_void,
            iov_len: buf.len(),
        };
        let remote_iov = libc::iovec {
            iov_base: address as *mut libc::c_void,
            iov_len: buf.len(),
        };
        let result = unsafe { libc::process_vm_writev(self.pid, &local_iov, 1, &remote_iov, 1, 0) };
        if result == -1 {
            anyhow::bail!(
                "process_vm_writev failed writing {} bytes at 0x{:x}: {}",
                buf.len(),
                address,
                std::io::Error::last_os_error()
            );
        }
        Ok(())
    }
}

#[cfg(target_os = "windows")]
impl Mem for ProcessHandle {
    fn read_buf(&self, address: u64, buf: &mut [u8]) -> Result<()> {
        use read_process_memory::{CopyAddress, Pid};
        let handle: read_process_memory::ProcessHandle = (self.pid as Pid).try_into()?;
        handle
            .copy_address(address as usize, buf)
            .with_context(|| format!("reading {} bytes at 0x{:x}", buf.len(), address))
    }

    fn write_buf(&self, address: u64, buf: &[u8]) -> Result<()> {
        use read_process_memory::Pid;
        use windows::Win32::Foundation::HANDLE;
        use windows::Win32::System::Diagnostics::Debug::WriteProcessMemory;
        let handle: read_process_memory::ProcessHandle = (self.pid as Pid).try_into()?;
        unsafe {
            WriteProcessMemory(
                HANDLE(*handle),
                address as *const _,
                buf.as_ptr() as *const _,
                buf.len(),
                None,
            )?;
        }
        Ok(())
    }
}

// --- Ctx ---

pub struct CtxInner {
    pub mem: Box<dyn Mem>,
    pub fnamepool: u64,
    pub structs: HashMap<String, StructInfo>,
    pub version: (u16, u16),
    pub case_preserving: bool,
    pub uobjectarray: u64,
    pub image_base_address: u64,
    pub build_change_list: Option<String>,
}

/// Shared context: single Arc clone per Ptr operation. Deref to `CtxInner` for field access.
#[derive(Clone)]
pub struct Ctx(Arc<CtxInner>);

impl Ctx {
    pub fn new(inner: CtxInner) -> Self {
        Self(Arc::new(inner))
    }

    pub fn read_buf(&self, address: u64, buf: &mut [u8]) -> Result<()> {
        self.mem.read_buf(address, buf)
    }
    pub fn read<T: Pod>(&self, address: u64) -> Result<T> {
        let mut buf = vec![0u8; std::mem::size_of::<T>()];
        self.mem.read_buf(address, &mut buf)?;
        T::try_from_bytes(&buf)
    }
    pub fn read_vec<T: Pod>(&self, address: u64, count: usize) -> Result<Vec<T>> {
        let size = std::mem::size_of::<T>();
        let mut buf = vec![0u8; count * size];
        self.mem.read_buf(address, &mut buf)?;
        let mut result = Vec::with_capacity(count);
        for i in 0..count {
            let start = i * size;
            let end = start + size;
            result.push(T::try_from_bytes(&buf[start..end])?);
        }
        Ok(result)
    }
    pub fn write_buf(&self, address: u64, buf: &[u8]) -> Result<()> {
        self.mem.write_buf(address, buf)
    }
    pub fn write<T: Pod>(&self, address: u64, value: &T) -> Result<()> {
        let bytes = unsafe {
            std::slice::from_raw_parts(value as *const T as *const u8, std::mem::size_of::<T>())
        };
        self.mem.write_buf(address, bytes)
    }
    pub fn clear_cache(&self) {
        self.mem.clear_cache();
    }

    pub fn get_struct(&self, struct_name: &str) -> &StructInfo {
        let Some(s) = self.structs.get(struct_name) else {
            panic!("struct {struct_name} not found");
        };
        s
    }
    pub fn struct_member(&self, struct_name: &str, member_name: &str) -> usize {
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
    pub fn ue_version(&self) -> (u16, u16) {
        self.version
    }
}

impl std::ops::Deref for Ctx {
    type Target = CtxInner;
    fn deref(&self) -> &CtxInner {
        &self.0
    }
}

// --- Ptr ---

#[derive(Clone)]
pub struct Ptr<T> {
    address: NonZero<u64>,
    ctx: Ctx,
    _type: PhantomData<T>,
}
impl<T> std::fmt::Debug for Ptr<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Ptr(0x{:x})", self.address)
    }
}
impl<T> Ptr<T> {
    pub fn new(address: u64, ctx: Ctx) -> Result<Self> {
        Ok(Self {
            address: address.try_into().context("unexpected null ptr")?,
            ctx,
            _type: Default::default(),
        })
    }
    pub fn new_non_zero(address: NonZero<u64>, ctx: Ctx) -> Self {
        Self {
            address,
            ctx,
            _type: Default::default(),
        }
    }
    pub fn ctx(&self) -> &Ctx {
        &self.ctx
    }
    pub fn address(&self) -> u64 {
        self.address.get()
    }
    pub fn map(&self, map: impl FnOnce(u64) -> u64) -> Result<Self> {
        Self::new(map(self.address.into()), self.ctx.clone())
    }
    pub fn cast<O>(&self) -> Ptr<O> {
        Ptr::new_non_zero(self.address, self.ctx.clone())
    }
    pub fn byte_offset(&self, n: usize) -> Self {
        Self::new_non_zero(
            self.address.checked_add(n as u64).unwrap(),
            self.ctx.clone(),
        )
    }
}
// offset for Pod types (known size at compile time)
impl<T: Pod> Ptr<T> {
    pub fn offset(&self, n: usize) -> Self {
        self.byte_offset(n * std::mem::size_of::<T>())
    }
    pub fn read(&self) -> Result<T> {
        self.ctx.read(self.address.into())
    }
    pub fn read_vec(&self, count: usize) -> Result<Vec<T>> {
        self.ctx.read_vec(self.address.into(), count)
    }
}
// offset for Ptr<Ptr<T>> (always 8 bytes)
impl<T> Ptr<Ptr<T>> {
    pub fn offset(&self, n: usize) -> Self {
        self.byte_offset(n * 8)
    }
    pub fn read(&self) -> Result<Ptr<T>> {
        let addr = self.ctx.read::<u64>(self.address.into())?;
        Ok(self.map(|_| addr)?.cast())
    }
}
// offset for Ptr<Option<Ptr<T>>> (always 8 bytes)
impl<T> Ptr<Option<Ptr<T>>> {
    pub fn offset(&self, n: usize) -> Self {
        self.byte_offset(n * 8)
    }
    pub fn read(&self) -> Result<Option<Ptr<T>>> {
        let addr = self.ctx.read::<u64>(self.address.into())?;
        Ok(if addr != 0 {
            Some(self.map(|_| addr)?.cast())
        } else {
            None
        })
    }
}
