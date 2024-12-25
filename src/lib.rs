mod containers;

#[cfg(test)]
mod test {
    use std::borrow::Cow;

    use anyhow::{Context, Result};
    use patternsleuth::{PatternConfig, resolvers::impl_try_collector, scanner::Pattern};
    use read_process_memory::{CopyAddress as _, Pid, ProcessHandle};

    use crate::containers::{
        EObjectFlags, ExternalPtr, FNamePool, FUObjectArray, Mem, PtrFNamePool, UObject,
    };

    impl_try_collector! {
        #[derive(Debug, PartialEq, Clone)]
        struct DrgResolution {
            guobject_array: patternsleuth::resolvers::unreal::guobject_array::GUObjectArray,
            fname_pool: patternsleuth::resolvers::unreal::fname::FNamePool,
        }
    }
    impl Mem for ProcessHandle {
        fn read_buf(&self, address: usize, buf: &mut [u8]) -> Result<()> {
            self.copy_address(address, buf)?;
            Ok(())
        }
    }

    #[test]
    fn test_drg() -> Result<()> {
        use bytemuck::Pod;

        let pid = 1490227;
        let mem: ProcessHandle = (pid as Pid).try_into()?;

        let results = patternsleuth::process::external::read_image_from_pid(pid)?
            .resolve(DrgResolution::resolver())?;

        let guobjectarray = ExternalPtr::<FUObjectArray>::new(results.guobject_array.0);
        //let fnamepool = ExternalPtr::<FNamePool>::new(results.fname_pool.0);
        let fnamepool = PtrFNamePool(results.fname_pool.0);

        println!("GUObjectArray = {guobjectarray:x?} FNamePool = {fnamepool:x?}");

        let uobject_array = guobjectarray.read(&mem)?;

        for i in 0..uobject_array.ObjObjects.NumElements {
            let obj = uobject_array.ObjObjects.read_item(&mem, i as usize)?;
            if obj.Object == 0 {
                continue;
            }
            let obj_base: UObject = mem.read(obj.Object)?;

            //if !obj_base.ObjectFlags.contains(EObjectFlags::RF_WasLoaded) {
            //    continue;
            //}
            let mut path = String::new();
            let name = fnamepool.read(&mem, obj_base.NamePrivate)?;
            if name == "FSDSaveGame" {
                println!("{name:?} {obj_base:x?}");
                if let Some(class) = obj_base.ClassPrivate.read_opt(&mem)? {
                    println!("{class:#?}");
                }
            }
            if name == "HealingCrystal_Light_C" {
                println!("{name:?} {obj_base:x?}");
                if let Some(class) = obj_base.ClassPrivate.read_opt(&mem)? {
                    println!("{class:#?}");
                    let data = &class.SuperFuncMap.base.base.pairs.elements.data;
                    let a = data.read(&mem)?;
                    println!("{a:#?}");
                    for t in &a {
                        println!(
                            "{:?} {:?}",
                            fnamepool.read(&mem, t.inner.Value.key)?,
                            t.inner.Value.value.read(&mem)?
                        );
                    }
                }
            }
        }

        Ok(())
    }
}
