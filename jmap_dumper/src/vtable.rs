use std::collections::{BTreeMap, HashMap, HashSet};

use crate::mem::Mem;
use anyhow::Result;
use jmap::{Address, ObjectType};

pub fn analyze_vtables<M: Mem>(
    mem: &M,
    objects: &mut BTreeMap<String, ObjectType>,
) -> BTreeMap<Address, Vec<Address>> {
    let mut class_vtables: HashMap<String, Address> = HashMap::new();
    let mut grouped: BTreeMap<Address, HashSet<&str>> = Default::default();
    for obj in objects.values() {
        let object = obj.get_object();
        let vtable = object.vtable;
        let class = object.class.as_str();
        class_vtables
            .insert(class.to_string(), vtable)
            .inspect(|existing| assert_eq!(*existing, vtable, "found conflicting vtable"));
        grouped.entry(vtable).or_default().insert(class);
    }

    // for (i, (vtable, classes)) in grouped.iter().enumerate() {
    //     println!("{i} {vtable:08x} {classes:?}");
    // }

    fn read_ptr<M: Mem>(mem: &M, addr: u64) -> Result<u64> {
        let mut buf = [0; 8];
        mem.read_buf(addr, &mut buf)?;
        Ok(u64::from_le_bytes(buf))
    }
    fn is_valid<M: Mem>(mem: &M, addr: u64) -> bool {
        // TODO check for executable bit, not just valid memory
        let mut buf = [0; 1];
        mem.read_buf(addr, &mut buf).is_ok()
    }

    let mut vtables: BTreeMap<Address, Vec<Address>> = Default::default();

    let mut vtable_iter = grouped.iter().peekable();
    while let Some((vtable, _classes)) = vtable_iter.next() {
        let next = vtable_iter.peek();

        let mut addr = *vtable;
        let mut funcs = vec![];

        // println!("Searching {addr:08x}");

        loop {
            if next.is_some_and(|(ptr, _)| addr >= **ptr) {
                // println!("BREAK NEXT n={}", funcs.len());
                break;
            }

            if let Ok(ptr) = read_ptr(mem, addr.0) {
                if is_valid(mem, ptr) {
                    funcs.push(ptr.into());
                } else {
                    // println!("BREAK BAD FUNC PTR n={}", funcs.len());
                    break;
                }
            } else {
                // println!("BREAK BAD READ n={}", funcs.len());
                break;
            }
            addr.0 += 8;
        }
        // println!("{classes:x?}");
        // println!("{funcs:x?}");

        assert!(vtables.insert(*vtable, funcs).is_none());
    }

    // trim vtables as they must be bounded by size of child vtable
    for (path, obj) in &*objects {
        if obj.get_class().is_some() {
            let mut class = path.as_str();
            let Some(vtable_ptr) = class_vtables.get(class) else {
                // println!("no vtable found for class {class}");
                continue;
            };
            let mut vtable_len = vtables.get(vtable_ptr).unwrap().len();

            while let Some(parent) = objects
                .get(class)
                .unwrap()
                .get_struct()
                .unwrap()
                .super_struct
                .as_deref()
            {
                class = parent;
                let Some(vtable_ptr) = class_vtables.get(class) else {
                    // println!("no vtable found for class {class}");
                    continue;
                };
                let vtable = vtables.get_mut(vtable_ptr).unwrap();
                if vtable.len() > vtable_len {
                    // println!(
                    //     "trimming vtable {} -> {} ({}) for {class}",
                    //     vtable.len(),
                    //     vtable_len,
                    //     vtable.len() - vtable_len
                    // );
                    vtable.truncate(vtable_len);
                }
                vtable_len = vtable.len();
            }
        }
    }

    // update UClass::instance_vtable
    for (class, vtable) in class_vtables {
        match objects.get_mut(&class).unwrap() {
            ObjectType::Class(class) => class.instance_vtable = Some(vtable),
            _ => unreachable!(),
        }
    }

    // {
    //     fn get_class<'a>(objects: &'a BTreeMap<String, ObjectType>, class: &str) -> &'a Class {
    //         objects.get(class).unwrap().get_class().unwrap()
    //     }
    //     // let mut class = "/Script/FSD.EnemyTemperatureComponent";
    //     // let mut class = "/Script/FSD.FSDGameInstance";
    //     // let mut class = "/Script/FSD.TagVanitySeasonalEvent";
    //     let mut class = "/Script/FSD.FSDGameMode";
    //     let vtable_ptr = get_class(objects, class).instance_vtable.unwrap();
    //     let vtable = vtables.get(&vtable_ptr).unwrap();
    //     println!("vtable_ptr={vtable_ptr:08x}");
    //     let mut funcs: Vec<(u64, &str)> = vtable.iter().map(|func| (*func, class)).collect();

    //     println!("hierarchy:");
    //     println!("{class}");
    //     while let Some(parent) = objects
    //         .get(class)
    //         .unwrap()
    //         .get_struct()
    //         .unwrap()
    //         .super_struct
    //         .as_deref()
    //     {
    //         class = parent;
    //         println!("{}", class);
    //         if let Some(vtable_ptr) = get_class(objects, class).instance_vtable {
    //             let vtable = vtables.get(&vtable_ptr).unwrap();
    //             for (i, func) in vtable.iter().enumerate() {
    //                 if funcs[i].0 == *func {
    //                     funcs[i].1 = class;
    //                 }
    //             }
    //         } else {
    //             println!("no vtable found for class {class}");
    //             break;
    //         }
    //     }

    //     for (i, (func, class)) in funcs.iter().enumerate() {
    //         println!("{i:>4} ptr={func:08x} owner={class}");
    //     }
    // }

    vtables
}
