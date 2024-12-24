#![feature(try_trait_v2, never_type)]
use std::ops::{ControlFlow, FromResidual, Try};

impl<T, E> Try for MaybeResult<T, E> {
    type Output = T;
    type Residual = MaybeResult<T, E>;
    fn from_output(output: Self::Output) -> Self {
        Self::Ok(output)
    }
    fn branch(self) -> ControlFlow<Self::Residual, Self::Output> {
        match self {
            MaybeResult::Ok(value) => ControlFlow::Continue(value),
            MaybeResult::Err(err) => ControlFlow::Break(MaybeResult::Err(err)),
        }
    }
}

impl<T, E> FromResidual<MaybeResult<T, E>> for MaybeResult<T, E> {
    fn from_residual(residual: MaybeResult<T, E>) -> Self {
        residual
    }
}
//impl<T, E> FromResidual<T> for MaybeResult<T, E> {
//    fn from_residual(residual: T) -> Self { todo!() }
//}

impl<T, E> FromResidual<MaybeResult<T, E>> for () {
    fn from_residual(_residual: MaybeResult<T, E>) -> Self {}
}
impl<T, E> FromResidual<MaybeResult<T, E>> for &() {
    fn from_residual(_residual: MaybeResult<T, E>) -> Self {
        todo!()
    }
}
impl<T, F, E> FromResidual<MaybeResult<T, E>> for Result<F, E> {
    fn from_residual(residual: MaybeResult<T, E>) -> Self {
        match residual {
            //MaybeResult::Ok(t) => Ok(t),
            MaybeResult::Err(e) => Err(e),
            _ => todo!("asdf"),
        }
    }
}

//impl<T, E> FromResidual<MaybeResult<T, E>> for <Infallible as FallibleTrait>::Output<&T> {
//    fn from_residual(_residual: MaybeResult<T, E>) -> Self {todo!()}
//}

//#[test]
//fn test_asdf() {
//    let tmp = MaybeResult::<usize, !>::Ok(1);
//    tmp?;
//}
//
//#[test]
//fn test_asdf4() {
//    let tmp = MaybeResult::<usize, !>::Err(!);
//    tmp?;
//}
//
//#[test]
//fn test_asdf2() -> Result<(), anyhow::Error> {
//    let tmp = MaybeResult::<usize, anyhow::Error>::Ok(1);
//    let size: usize = tmp?;
//    Ok(())
//}
//
//#[test]
//fn test_asdf3() -> Result<(), anyhow::Error> {
//    let tmp = MaybeResult::<usize, anyhow::Error>::Err(anyhow::anyhow!("uhh"));
//    let size: usize = tmp?;
//    Ok(())
//}

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
        F::unwrap(self.mem.read(self.ptr, &mut tmp))?; // TODO how to propagate any errors?
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
        //F::unwrap(self.mem.read(self.ptr + 10, &mut tmp))?;
        F::wrap(*unsafe { tmp.assume_init() })
    }
}

enum MaybeResult<T, E> {
    Ok(T),
    Err(E),
}

//impl<T> FromResidual<MaybeResult<T, !>> for MaybeResult<T, !> {
//    fn from_residual(residual: <Self as Try>::Residual) -> Self {
//        todo!()
//    }
//}

trait FallibleTrait {
    type Output<U>;
    type Err;
    fn wrap<U>(value: U) -> Self::Output<U>;
    fn unwrap<U>(value: Self::Output<U>) -> MaybeResult<U, Self::Err>;
}

struct Infallible;
impl FallibleTrait for Infallible {
    type Output<U> = U;
    type Err = !;
    fn wrap<U>(value: U) -> U {
        value
    }
    fn unwrap<U>(value: Self::Output<U>) -> MaybeResult<U, Self::Err> {
        MaybeResult::Ok(value)
    }
}

struct Fallible<E>(PhantomData<E>);
impl<E> FallibleTrait for Fallible<E> {
    type Output<U> = Result<U, E>;
    type Err = E;
    fn wrap<U>(value: U) -> Result<U, E> {
        Ok(value)
    }
    fn unwrap<U>(value: Self::Output<U>) -> MaybeResult<U, Self::Err> {
        match value {
            Ok(t) => MaybeResult::Ok(t),
            Err(e) => MaybeResult::Err(e),
        }
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
