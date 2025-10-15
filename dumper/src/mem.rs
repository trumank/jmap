use crate::{containers::PtrFNamePool, structs::StructInfo};
use anyhow::{Context as _, Result};
use read_process_memory::{CopyAddress as _, ProcessHandle};
use std::{
    collections::HashMap,
    marker::PhantomData,
    num::NonZero,
    sync::{Arc, Mutex},
};
use ue_reflection::{
    EClassCastFlags, EClassFlags, ECppForm, EEnumFlags, EFunctionFlags, EObjectFlags,
    EPropertyFlags, EStructFlags,
};

pub trait VirtSize<C: Ctx> {
    fn size(ctx: &C) -> usize;
}

#[derive(Clone)]
#[repr(C)]
pub struct Ptr<T, C> {
    address: NonZero<u64>,
    ctx: C,
    _type: PhantomData<T>,
}
impl<T, C> std::fmt::Debug for Ptr<T, C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Ptr(0x{:x})", self.address)
    }
}
impl<T, C> Ptr<T, C> {
    pub fn new(address: u64, ctx: C) -> Self {
        Self {
            address: address.try_into().unwrap(),
            ctx,
            _type: Default::default(),
        }
    }
    pub fn new_non_zero(address: NonZero<u64>, ctx: C) -> Self {
        Self {
            address,
            ctx,
            _type: Default::default(),
        }
    }
    pub fn ctx(&self) -> &C {
        &self.ctx
    }
    pub fn address(&self) -> u64 {
        self.address.get()
    }
}
impl<T, C: Clone> Ptr<T, C> {
    pub fn map(&self, map: impl FnOnce(u64) -> u64) -> Self {
        Self::new(map(self.address.into()), self.ctx.clone())
    }
    pub fn cast<O>(&self) -> Ptr<O, C> {
        Ptr::new_non_zero(self.address, self.ctx.clone())
    }
    pub fn byte_offset(&self, n: usize) -> Self {
        Self::new_non_zero(
            self.address.checked_add(n as u64).unwrap(),
            self.ctx.clone(),
        )
    }
}
impl<T: VirtSize<C>, C: Clone + Ctx> Ptr<T, C> {
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
impl<T, C: Mem> Ptr<Option<Ptr<T, C>>, C> {
    pub fn read(&self) -> Result<Option<Ptr<T, C>>> {
        let addr = self.ctx.read::<u64>(self.address.into())?;
        Ok(if addr != 0 {
            Some(self.map(|_| addr).cast())
        } else {
            None
        })
    }
}
impl<T, C: Mem> Ptr<Ptr<T, C>, C> {
    pub fn read(&self) -> Result<Ptr<T, C>> {
        let addr = self.ctx.read::<u64>(self.address.into())?;
        Ok(self.map(|_| addr).cast())
    }
}

pub trait TryFromBytes: Sized {
    fn try_from_bytes(bytes: &[u8]) -> Result<Self>;
}

pub trait Pod: TryFromBytes {}

macro_rules! impl_try_from_bytes_pod {
    ($($t:ty),* $(,)?) => {
        $(
            impl TryFromBytes for $t {
                fn try_from_bytes(bytes: &[u8]) -> Result<Self> {
                    Ok(bytemuck::pod_read_unaligned(bytes))
                }
            }
        )*
    };
}

macro_rules! impl_try_from_bytes_bitflags {
    ($(($t:ty, $bits_ty:ty)),* $(,)?) => {
        $(
            impl TryFromBytes for $t {
                fn try_from_bytes(bytes: &[u8]) -> Result<Self> {
                    let bits: $bits_ty = bytemuck::pod_read_unaligned(bytes);
                    Self::from_bits(bits)
                        .ok_or_else(|| anyhow::anyhow!("Invalid {} bits: 0x{:x}", stringify!($t), bits))
                }
            }
        )*
    };
}

impl_try_from_bytes_pod!(i8, u8, i16, u16, i32, u32, i64, u64, usize, f32, f64);

impl_try_from_bytes_bitflags!(
    (EObjectFlags, u32),
    (EClassCastFlags, u64),
    (EClassFlags, u32),
    (EFunctionFlags, u32),
    (EStructFlags, u32),
    (EPropertyFlags, u64),
    (EEnumFlags, u8),
);

impl TryFromBytes for ECppForm {
    fn try_from_bytes(bytes: &[u8]) -> Result<Self> {
        let discriminant: u8 = bytemuck::pod_read_unaligned(bytes);
        Self::from_repr(discriminant)
            .ok_or_else(|| anyhow::anyhow!("Invalid ECppForm discriminant: {}", discriminant))
    }
}

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

impl<T: Pod, C: Ctx> VirtSize<C> for T {
    fn size(_ctx: &C) -> usize {
        std::mem::size_of::<Self>()
    }
}

impl<T, C: Ctx> VirtSize<C> for Ptr<T, C> {
    fn size(_ctx: &C) -> usize {
        8
    }
}
impl<T, C: Ctx> VirtSize<C> for Option<Ptr<T, C>> {
    fn size(_ctx: &C) -> usize {
        8
    }
}

pub trait Mem: Clone {
    fn read_buf(&self, address: u64, buf: &mut [u8]) -> Result<()>;
    fn read<T: Pod>(&self, address: u64) -> Result<T> {
        let mut buf = vec![0u8; std::mem::size_of::<T>()];
        self.read_buf(address, &mut buf)?;
        T::try_from_bytes(&buf)
    }

    fn read_vec<T: Pod>(&self, address: u64, count: usize) -> Result<Vec<T>> {
        let size = std::mem::size_of::<T>();
        let mut buf = vec![0u8; count * size];
        self.read_buf(address, &mut buf)?;
        let mut result = Vec::with_capacity(count);
        for i in 0..count {
            let start = i * size;
            let end = start + size;
            result.push(T::try_from_bytes(&buf[start..end])?);
        }
        Ok(result)
    }
}
const PAGE_SIZE: usize = 0x1000;
#[derive(Clone)]
pub struct MemCache<M> {
    inner: M,
    pages: Arc<Mutex<HashMap<u64, Vec<u8>>>>,
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
}

impl Mem for ProcessHandle {
    fn read_buf(&self, address: u64, buf: &mut [u8]) -> Result<()> {
        self.copy_address(address as usize, buf)
            .with_context(|| format!("reading {} bytes at 0x{:x}", buf.len(), address))
    }
}

pub trait Ctx: Mem {
    fn fnamepool(&self) -> PtrFNamePool;
    fn get_struct(&self, struct_name: &str) -> &StructInfo;
    fn struct_member(&self, struct_name: &str, member_name: &str) -> usize;
    fn ue_version(&self) -> (u16, u16);
    fn case_preserving(&self) -> bool;
}

#[derive(Clone)]
pub struct CtxPtr<M: Mem> {
    pub mem: M,
    pub fnamepool: PtrFNamePool,
    pub structs: Arc<HashMap<String, StructInfo>>,
    pub version: (u16, u16),
    pub case_preserving: bool,
}
impl<M: Mem> Mem for CtxPtr<M> {
    fn read_buf(&self, address: u64, buf: &mut [u8]) -> Result<()> {
        self.mem.read_buf(address, buf)
    }
}
impl<M: Mem> Ctx for CtxPtr<M> {
    fn fnamepool(&self) -> PtrFNamePool {
        self.fnamepool
    }
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
    fn ue_version(&self) -> (u16, u16) {
        self.version
    }
    fn case_preserving(&self) -> bool {
        self.case_preserving
    }
}
