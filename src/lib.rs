use std::{cell::UnsafeCell, marker::PhantomData, mem::MaybeUninit};

type Ptr = usize;

trait Memory<F: FallibleTrait> {
    fn read<T>(&self, ptr: Ptr, buf: &mut T) -> F::Output<()>;
}
struct MemoryLocal;
impl Memory<Infallible> for MemoryLocal {
    fn read<T>(&self, ptr: Ptr, buf: &mut T) {
        todo!()
    }
}

struct MemoryRemote(u32);
impl Memory<Fallible<anyhow::Error>> for MemoryRemote {
    fn read<T>(&self, ptr: Ptr, buf: &mut T) -> Result<(), anyhow::Error> {
        todo!()
    }
}

struct Container<T, M: Memory<F>, F: FallibleTrait = Infallible> {
    mem: M,
    ptr: Ptr,
    _phantom_t: PhantomData<T>,
    _phantom: PhantomData<F>,
}

impl<T, M: Memory<F>, F: FallibleTrait> Container<T, M, F> {
    fn new(mem: M, ptr: Ptr) -> Self {
        Self {
            mem,
            ptr,
            _phantom_t: Default::default(),
            _phantom: Default::default(),
        }
    }
    fn get(&self) -> F::Output<&T> {
        let mut tmp: MaybeUninit<&T> = MaybeUninit::uninit();
        self.mem.read(self.ptr, &mut tmp); // TODO how to propagate any errors?
        F::wrap(unsafe { tmp.assume_init() })
    }
}

struct BasicBitch<M: Memory<F>, F: FallibleTrait> {
    ptr: Ptr,
    mem: M,
    _phantom_f: PhantomData<F>,
}
impl<M: Memory<F>, F: FallibleTrait> BasicBitch<M, F> {
    fn int_member(&self) -> F::Output<i32> {
        let mut tmp: MaybeUninit<&i32> = MaybeUninit::uninit();
        self.mem.read(self.ptr + 10, &mut tmp);
        F::wrap(*unsafe { tmp.assume_init() })
    }
}

trait FallibleTrait {
    type Output<U>;
    fn wrap<U>(value: U) -> Self::Output<U>;
}

struct Infallible;
impl FallibleTrait for Infallible {
    type Output<U> = U;
    fn wrap<U>(value: U) -> U {
        value
    }
}

struct Fallible<E>(PhantomData<E>);
impl<E> FallibleTrait for Fallible<E> {
    type Output<U> = Result<U, E>;
    fn wrap<U>(value: U) -> Result<U, E> {
        Ok(value)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test() -> Result<(), Box<dyn std::error::Error>> {
        let mem_local = MemoryLocal;
        let mem_remote = MemoryRemote(1234);
        let c = Container::new(mem_local, 1);
        let c_f = Container::new(mem_remote, 1);
        let a: i32 = *c_f.get()?;
        let a: i32 = *c.get();

        Ok(())
    }
}
