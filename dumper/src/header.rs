use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt::{write, Write};

use ue_reflection::{EClassCastFlags, ObjectType, Property, PropertyType};

type Objects = BTreeMap<String, ObjectType>;

type CTypes<'a> = HashSet<CType<'a>>;

struct GenCtx<'a> {
    types: CTypes<'a>,
}

#[derive(Default)]
struct TypeStore<'a> {
    next: usize,
    types: HashMap<TypeId, CType<'a>>,
    types_reverse: HashMap<CType<'a>, TypeId>,
}

impl<'a> TypeStore<'a> {
    fn insert(&mut self, type_: CType<'a>) -> TypeId {
        if let Some(existing) = self.types_reverse.get(&type_) {
            *existing
        } else {
            let id = TypeId(self.next);
            self.types.insert(id, type_.clone());
            self.types_reverse.insert(type_, id);
            self.next += 1;
            id
        }
    }
    fn get(&self, id: TypeId) -> Option<&CType<'a>> {
        self.types.get(&id)
    }
}
impl<'a> std::ops::Index<TypeId> for TypeStore<'a> {
    type Output = CType<'a>;

    fn index(&self, id: TypeId) -> &Self::Output {
        self.get(id).unwrap()
    }
}

fn obj_name(objects: &Objects, path: &str) -> String {
    let obj = &objects[path];
    let name = path.rsplit(['/', '.', ':']).next().unwrap();
    match obj {
        //ue_reflection::ObjectType::Object(object) => todo!(),
        //ue_reflection::ObjectType::Package(package) => todo!(),
        ue_reflection::ObjectType::Enum(_) => name.into(),
        ue_reflection::ObjectType::ScriptStruct(_script_struct) => {
            format!("F{name}")
        }
        ue_reflection::ObjectType::Class(class) => {
            let is_actor = class
                .class_cast_flags
                .contains(EClassCastFlags::CASTCLASS_AActor);
            if class.r#struct.super_struct.as_deref() == Some("/Script/CoreUObject.Interface") {
                format!("I{name}")
            } else if is_actor {
                format!("A{name}")
            } else {
                format!("U{name}")
            }
        }
        //ue_reflection::ObjectType::Function(function) => todo!(),
        _ => todo!("{path} {obj:?}"),
    }
}

pub fn into_header(
    objects: &Objects,
    filter: impl Fn(&str, &ue_reflection::ObjectType) -> bool,
) -> String {
    let mut buffer = String::new();

    let mut type_store = TypeStore::default();

    //for (path, obj) in objects {
    //    match obj {
    //        //ue_reflection::ObjectType::Object(object) => todo!(),
    //        //ue_reflection::ObjectType::Package(package) => todo!(),
    //        //ue_reflection::ObjectType::Enum(_) => todo!(),
    //        //ue_reflection::ObjectType::ScriptStruct(script_struct) => todo!(),
    //        ue_reflection::ObjectType::Class(class) => {
    //            if filter(path, obj) {
    //                writeln!(&mut buffer, "class {} {{", obj_name(objects, path)).unwrap();

    //                for prop in &class.r#struct.properties {
    //                    let type_id = into_ctype(objects, prop, &mut type_store);
    //                    let type_name = type_to_string(objects, &type_store, type_id);
    //                    writeln!(
    //                        &mut buffer,
    //                        "    {} {}; // 0x{:x}",
    //                        type_name, prop.name, prop.offset
    //                    )
    //                    .unwrap();
    //                }

    //                writeln!(&mut buffer, "}}").unwrap();
    //            }
    //        }
    //        //ue_reflection::ObjectType::Function(function) => todo!(),
    //        _ => {}
    //    }
    //}

    let mut to_visit = HashSet::new();
    let mut dep_graph = HashMap::new();
    // get dependencies of initial top level classes
    for (path, obj) in objects {
        if filter(path, obj) {
            if let Some(_) = obj.get_class() {
                let mut dependencies = vec![];
                let class_id = type_store.insert(CType::UEClass(path));
                get_type_dependencies(&mut dependencies, objects, &mut type_store, class_id);
                let type_ = (DepType::Full, class_id);
                dep_graph.insert(type_, dependencies.clone());
                to_visit.extend(dependencies);
            }
        }
    }

    // get dependencies of dependencies
    loop {
        let visit = std::mem::take(&mut to_visit);
        if visit.is_empty() {
            break;
        }
        for type_ in visit.into_iter() {
            let mut dependencies = vec![];
            if !dep_graph.contains_key(&type_) {
                get_type_dependencies(&mut dependencies, objects, &mut type_store, type_.1);
                to_visit.extend(dependencies.clone());
            }
            dep_graph.insert(type_, dependencies.clone());
        }
    }

    dbg!(&dep_graph);
    dbg!(&type_store.types);

    // foward declarations
    for (dep_type, type_id) in dep_graph.keys() {
        if *dep_type == DepType::Partial {
            let type_ = &type_store[*type_id];
            match type_ {
                //CType::FName => todo!(),
                //CType::FString => todo!(),
                //CType::FText => todo!(),
                //CType::TArray(type_id) => todo!(),
                //CType::TMap(type_id, type_id1) => todo!(),
                //CType::TSet(type_id) => todo!(),
                //CType::Ptr(type_id) => todo!(),
                //CType::TWeakObjectPtr(type_id) => todo!(),
                //CType::TSoftObjectPtr(type_id) => todo!(),
                //CType::TScriptInterface(type_id) => todo!(),
                //CType::TTuple(type_id, type_id1) => todo!(),
                CType::UEClass(path) => {
                    writeln!(&mut buffer, "class {};", obj_name(objects, path)).unwrap();
                }
                CType::UEEnum(_) => todo!(),
                CType::UEStruct(path) => {
                    writeln!(&mut buffer, "struct {};", obj_name(objects, path)).unwrap();
                }
                _ => {}
            }
        }
    }

    let sorted = topological_sort(&dep_graph).unwrap();
    dbg!(&sorted);

    // full declarations
    for (dep_type, type_id) in &sorted {
        //if *dep_type == DepType::Full {
        decl_ctype(&mut buffer, objects, &mut type_store, *type_id);
        //}
    }

    buffer
}

fn into_ctype<'a>(objects: &'a Objects, prop: &'a Property, store: &mut TypeStore<'a>) -> TypeId {
    assert_eq!(prop.array_dim, 1, "TODO array_dim != 1");
    let new_type = match &prop.r#type {
        PropertyType::Struct { r#struct } => CType::UEStruct(r#struct),
        PropertyType::Str => CType::FString,
        PropertyType::Name => CType::FName,
        PropertyType::Text => CType::FText,
        PropertyType::MulticastInlineDelegate => CType::MulticastInlineDelegate, // TODO
        PropertyType::MulticastSparseDelegate => CType::MulticastSparseDelegate, // TODO
        PropertyType::Delegate => todo!(),
        PropertyType::Bool {
            field_size,
            byte_offset,
            byte_mask,
            field_mask,
        } => CType::Bool, // TODO
        PropertyType::Array { inner } => CType::TArray(into_ctype(objects, inner, store)),
        PropertyType::Enum { container, r#enum } => {
            CType::UEEnum(r#enum.as_ref().expect("TODO unknown enum name"))
        }
        PropertyType::Map {
            key_prop,
            value_prop,
        } => {
            let key = into_ctype(objects, key_prop, store);
            let value = into_ctype(objects, value_prop, store);
            CType::TMap(key, value)
        }
        PropertyType::Set { key_prop } => CType::TSet(into_ctype(objects, key_prop, store)),
        PropertyType::Float => CType::Float,
        PropertyType::Double => CType::Double,
        PropertyType::Byte { r#enum } => CType::Byte,
        PropertyType::UInt16 => CType::UInt16,
        PropertyType::UInt32 => CType::UInt32,
        PropertyType::UInt64 => CType::UInt64,
        PropertyType::Int8 => CType::Int8,
        PropertyType::Int16 => CType::Int16,
        PropertyType::Int => CType::Int32,
        PropertyType::Int64 => CType::Int64,
        PropertyType::Object { class } => {
            let class = CType::UEClass(class.as_deref().expect("expected class name"));
            CType::Ptr(store.insert(class))
        }
        PropertyType::WeakObject { class } => {
            let class = CType::UEClass(class);
            CType::TWeakObjectPtr(store.insert(class))
        }
        PropertyType::SoftObject { class } => {
            let class = CType::UEClass(class);
            CType::TSoftObjectPtr(store.insert(class))
        }
        PropertyType::LazyObject { class } => todo!(),
        PropertyType::Interface { class } => {
            let class = CType::UEClass(class);
            CType::TScriptInterface(store.insert(class))
        }
        PropertyType::FieldPath => todo!(),
        PropertyType::Optional { inner } => todo!(),
    };
    store.insert(new_type)
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct TypeId(usize);
impl std::fmt::Debug for TypeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "TypeId({})", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum CType<'a> {
    Float,
    Double,
    Byte,
    UInt16,
    UInt32,
    UInt64,
    Int8,
    Int16,
    Int32,
    Int64,

    Bool, // TODO bitfield

    FName,
    FString,
    FText,
    MulticastInlineDelegate,
    MulticastSparseDelegate,

    TArray(TypeId),
    TMap(TypeId, TypeId),
    TSet(TypeId),
    Ptr(TypeId),
    TWeakObjectPtr(TypeId),
    TSoftObjectPtr(TypeId),
    TScriptInterface(TypeId),

    TTuple(TypeId, TypeId),

    UEClass(&'a str),
    UEEnum(&'a str),
    UEStruct(&'a str),
}

fn type_to_string(objects: &Objects, store: &TypeStore<'_>, id: TypeId) -> String {
    let ctype = &store[id];
    match ctype {
        CType::Float => "float".into(),
        CType::Double => "double".into(),
        CType::Byte => "uint8_t".into(), // TODO enum
        CType::UInt16 => "uint16_t".into(),
        CType::UInt32 => "uint32_t".into(),
        CType::UInt64 => "uint64_t".into(),
        CType::Int8 => "int8_t".into(),
        CType::Int16 => "int16_t".into(),
        CType::Int32 => "int32_t".into(),
        CType::Int64 => "int64_t".into(),

        CType::Bool => "bool".into(),

        CType::FName => "FName".into(),
        CType::FString => "FString".into(),
        CType::FText => "FText".into(),
        CType::MulticastInlineDelegate => "MulticastInlineDelegate".into(),
        CType::MulticastSparseDelegate => "MulticastSparseDelegate".into(),

        CType::TArray(type_id) => format!("TArray<{}>", type_to_string(objects, store, *type_id)),
        CType::TMap(k, v) => format!(
            "TMap<{}, {}>",
            type_to_string(objects, store, *k),
            type_to_string(objects, store, *v)
        ),
        CType::TSet(type_id) => format!("TSet<{}>", type_to_string(objects, store, *type_id)),
        CType::Ptr(type_id) => format!("{}*", type_to_string(objects, store, *type_id)),
        CType::TWeakObjectPtr(type_id) => {
            format!(
                "TWeakObjectPtr<{}>",
                type_to_string(objects, store, *type_id)
            )
        }
        CType::TSoftObjectPtr(type_id) => {
            format!(
                "TSoftObjectPtr<{}>",
                type_to_string(objects, store, *type_id)
            )
        }
        CType::TScriptInterface(type_id) => {
            format!(
                "TScriptInterface<{}>",
                type_to_string(objects, store, *type_id)
            )
        }

        CType::TTuple(a, b) => {
            let a = type_to_string(objects, store, *a);
            let b = type_to_string(objects, store, *b);
            format!("TTuple<{a}, {b}>",)
        }

        CType::UEClass(path) => obj_name(objects, path),
        CType::UEEnum(path) => obj_name(objects, path),
        CType::UEStruct(path) => obj_name(objects, path),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum DepType {
    Partial,
    Full,
}

fn get_type_dependencies<'a>(
    dependencies: &mut Vec<(DepType, TypeId)>,
    objects: &'a Objects,
    store: &mut TypeStore<'a>,
    id: TypeId,
) {
    let ctype = store[id].clone();
    match ctype {
        CType::Float => {}
        CType::Double => {}
        CType::Byte => {}
        CType::UInt16 => {}
        CType::UInt32 => {}
        CType::UInt64 => {}
        CType::Int8 => {}
        CType::Int16 => {}
        CType::Int32 => {}
        CType::Int64 => {}
        CType::Bool => {}
        CType::FName => {}
        CType::FString => {
            let inner = store.insert(CType::UInt16); // TODO wchar
            let array = store.insert(CType::TArray(inner));
            dependencies.push((DepType::Full, array));
        }
        CType::FText => {}
        CType::MulticastInlineDelegate => {}
        CType::MulticastSparseDelegate => {}
        CType::TArray(type_id) => {
            dependencies.push((DepType::Full, type_id));
        }
        CType::TMap(k, v) => {
            let tuple = store.insert(CType::TTuple(k, v));
            dependencies.push((DepType::Full, tuple));
            dependencies.push((DepType::Full, k));
            dependencies.push((DepType::Full, v));
        }
        CType::TSet(type_id) => {}
        CType::Ptr(type_id) => {
            dependencies.push((DepType::Partial, type_id));
        }
        CType::TWeakObjectPtr(type_id) => {}
        CType::TSoftObjectPtr(type_id) => {}
        CType::TScriptInterface(type_id) => {}
        CType::TTuple(a, b) => {
            dependencies.push((DepType::Full, a));
            dependencies.push((DepType::Full, b));
        }
        CType::UEClass(class) => {
            let class = &objects[class].get_class().unwrap();
            for prop in &class.r#struct.properties {
                let prop_id = into_ctype(objects, prop, store);
                dependencies.push((DepType::Full, prop_id));
                get_type_dependencies(dependencies, objects, store, prop_id);
            }
        }
        CType::UEEnum(_) => {}
        CType::UEStruct(struct_) => {
            let struct_ = &objects[struct_].get_struct().unwrap();
            for prop in &struct_.properties {
                let prop_id = into_ctype(objects, prop, store);
                dependencies.push((DepType::Full, prop_id));
                get_type_dependencies(dependencies, objects, store, prop_id);
            }
        }
    }
}

fn decl_ctype<'a>(
    buffer: &mut String,
    objects: &'a Objects,
    store: &mut TypeStore<'a>,
    id: TypeId,
) {
    let ctype = &store[id];
    match ctype {
        CType::Float => {}
        CType::Double => {}
        CType::Byte => {}
        CType::UInt16 => {}
        CType::UInt32 => {}
        CType::UInt64 => {}
        CType::Int8 => {}
        CType::Int16 => {}
        CType::Int32 => {}
        CType::Int64 => {}
        CType::Bool => {}
        CType::FName => {}
        CType::FString => {
            // TODO user TArray
            writeln!(
                buffer,
                r#"struct FString {{
    data: wchar_t*,
    num: int32_t,
    max: int32_t,
}};"#
            )
            .unwrap();
        }
        CType::FText => {
            writeln!(buffer, r#"struct FText {{ /* TODO */ }};"#).unwrap();
        }
        CType::MulticastInlineDelegate => {}
        CType::MulticastSparseDelegate => {}
        CType::TArray(type_id) => {
            let inner_name = type_to_string(objects, store, *type_id);
            writeln!(
                buffer,
                r#"struct `TArray<{0}>` {{
    {0} data;
    int32_t num;
    int32_t max;
}};"#,
                inner_name
            )
            .unwrap();
        }
        CType::TMap(k, v) => {
            let key_name = type_to_string(objects, store, *k);
            let value_name = type_to_string(objects, store, *v);

            // struct TSet<TTuple<int,FGeneratedMissionGroup>,TDefaultMapHashableKeyFuncs<int,FGeneratedMissionGroup,0>,FDefaultSetAllocator>  {
            // /* offset 0x000 */ Elements: TSparseArray<TSetElement<TTuple<int,FGeneratedMissionGroup> >,TSparseArrayAllocator<TSizedDefaultAllocator<32>,FDefaultBitArrayAllocator> >,
            // /* offset 0x038 */ Hash: TInlineAllocator<1,TSizedDefaultAllocator<32> >::ForElementType<FSetElementId>,
            // /* offset 0x048 */ HashSize: i32,

            writeln!(
                buffer,
                r#"struct `TMap<{0}, {1}>` {{
    // TODO
}};"#,
                key_name, value_name,
            )
            .unwrap();
        }
        CType::TSet(type_id) => {}
        CType::Ptr(type_id) => {}
        CType::TWeakObjectPtr(type_id) => {}
        CType::TSoftObjectPtr(type_id) => {}
        CType::TScriptInterface(type_id) => {}

        CType::TTuple(a, b) => {
            let a = type_to_string(objects, store, *a);
            let b = type_to_string(objects, store, *b);

            writeln!(
                buffer,
                r#"struct `TTuple<{0}, {1}>` {{
    {0} a;
    {1} b;
}};"#,
                a, b,
            )
            .unwrap();
        }

        CType::UEClass(path) => {
            writeln!(buffer, "class {} {{", obj_name(objects, path)).unwrap();
            let class = &objects[*path].get_class().unwrap();
            for prop in &class.r#struct.properties {
                let ctype = into_ctype(objects, prop, store);
                let type_name = type_to_string(objects, store, ctype);
                writeln!(buffer, "    `{}` {};", type_name, prop.name).unwrap();
            }
            writeln!(buffer, "}};").unwrap();
        }
        CType::UEEnum(_) => {}
        CType::UEStruct(path) => {
            writeln!(buffer, "struct {} {{", obj_name(objects, path)).unwrap();
            let struct_ = &objects[*path].get_struct().unwrap();
            for prop in &struct_.properties {
                let ctype = into_ctype(objects, prop, store);
                let type_name = type_to_string(objects, store, ctype);
                writeln!(buffer, "    `{}` {};", type_name, prop.name).unwrap();
            }
            writeln!(buffer, "}};").unwrap();
        }
    }
}

fn quote_name(name: &str) -> String {
    format!("`{name}`")
}

trait GraphKey: Clone + Copy + PartialEq + Eq + std::hash::Hash {}
impl<T> GraphKey for T where T: Clone + Copy + PartialEq + Eq + std::hash::Hash {}
fn topological_sort<T: GraphKey>(graph: &HashMap<T, Vec<T>>) -> Option<Vec<T>> {
    let mut result: Vec<T> = Vec::new();
    let mut visited = HashSet::new();
    let mut temp_visited = HashSet::new();

    // Function for DFS
    fn dfs<T: GraphKey>(
        node: T,
        graph: &HashMap<T, Vec<T>>,
        visited: &mut HashSet<T>,
        temp_visited: &mut HashSet<T>,
        result: &mut Vec<T>,
    ) -> bool {
        // If node is temporarily visited, we have a cycle
        if temp_visited.contains(&node) {
            return false;
        }

        // If node is already visited, skip
        if visited.contains(&node) {
            return true;
        }

        // Mark as temporarily visited
        temp_visited.insert(node);

        // Visit all neighbors
        if let Some(neighbors) = graph.get(&node) {
            for &neighbor in neighbors {
                if !dfs(neighbor, graph, visited, temp_visited, result) {
                    return false;
                }
            }
        }

        // Mark as visited and add to result
        temp_visited.remove(&node);
        visited.insert(node);
        result.push(node);

        true
    }

    // Run DFS for each node
    for node in graph.keys() {
        if !visited.contains(node) {
            if !dfs(
                node.clone(),
                graph,
                &mut visited,
                &mut temp_visited,
                &mut result,
            ) {
                return None; // Graph has a cycle
            }
        }
    }

    Some(result)
}

#[cfg(test)]
mod test {
    use super::*;

    use anyhow::Result;

    #[test]
    fn test_into_header() -> Result<()> {
        let objects: Objects = serde_json::from_slice(&std::fs::read("../fsd.json")?)?;
        let header = into_header(&objects, |path, obj| {
            path.contains("MissionGenerationManager")
        });
        println!("{header}");
        Ok(())
    }
}
