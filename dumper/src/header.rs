use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt::{write, Write};

use ue_reflection::{EClassCastFlags, ObjectType, Property, PropertyType, Struct};

use crate::objects::UEnum;

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

    let mut to_visit = vec![];
    let mut dep_graph = HashMap::new();
    // get dependencies of initial top level classes
    for (path, obj) in objects {
        if filter(path, obj) {
            if let Some(_) = obj.get_class() {
                let mut dependencies = vec![];
                let class_id = type_store.insert(CType::UEClass(path));
                let type_ = (DepType::Full, class_id);
                get_type_dependencies(&mut dependencies, objects, &mut type_store, type_);
                dep_graph.insert(type_, dependencies.clone());
                for dep in dependencies {
                    if !dep_graph.contains_key(&dep) && !to_visit.contains(&dep) {
                        to_visit.push(dep);
                        //if dep.0 == DepType::Partial {
                        //    let full = (DepType::Full, dep.1);
                        //    if !dep_graph.contains_key(&full) {
                        //        to_visit.push(full);
                        //    }
                        //}
                    }
                }
            }
        }
    }

    // get dependencies of dependencies
    while let Some(next) = to_visit.pop() {
        let mut dependencies = vec![];
        if !dep_graph.contains_key(&next) {
            get_type_dependencies(&mut dependencies, objects, &mut type_store, next);
        }
        assert!(dep_graph.insert(next, dependencies.clone()).is_none());
        for dep in dependencies {
            if !dep_graph.contains_key(&dep) && !to_visit.contains(&dep) {
                to_visit.push(dep);
                //if dep.0 == DepType::Partial {
                //    let full = (DepType::Full, dep.1);
                //    if !dep_graph.contains_key(&full) {
                //        to_visit.push(full);
                //    }
                //}
            }
        }
    }

    dbg!(&dep_graph);
    dbg!(&type_store.types);

    for (owner, dependencies) in &dep_graph {
        println!(
            "{:?} {}",
            owner,
            type_to_string(&objects, &type_store, owner.1, false)
        );
        for dep in dependencies {
            println!(
                "  {:?} {}",
                dep,
                type_to_string(&objects, &type_store, dep.1, false)
            );
        }
    }

    // forward declarations
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
                CType::UEEnum(_) => {}
                CType::UEStruct(path) => {
                    writeln!(&mut buffer, "struct `{}`;", obj_name(objects, path)).unwrap();
                }
                CType::UEClass(path) => {
                    writeln!(&mut buffer, "class `{}`;", obj_name(objects, path)).unwrap();
                }
                _ => {}
            }
        }
    }

    let sorted = topological_sort(&dep_graph).unwrap();
    dbg!(&sorted);

    // full declarations
    for (dep_type, type_id) in &sorted {
        if *dep_type == DepType::Full {
            decl_ctype(&mut buffer, objects, &mut type_store, *type_id);
        }
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
        PropertyType::Delegate => CType::Delegate,
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
        PropertyType::LazyObject { class } => {
            let class = CType::UEClass(class);
            CType::TLazyObjectPtr(store.insert(class))
        }
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
    Delegate,

    TArray(TypeId),
    TMap(TypeId, TypeId),
    TSet(TypeId),
    Ptr(TypeId),
    TWeakObjectPtr(TypeId),
    TSoftObjectPtr(TypeId),
    TLazyObjectPtr(TypeId),
    TScriptInterface(TypeId),

    TTuple(TypeId, TypeId),

    UEEnum(&'a str),
    UEStruct(&'a str),
    UEClass(&'a str),
}

struct TypeName {
    name: String,
    primitive: bool,
    pointer: bool,
}
impl TypeName {
    fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            primitive: false,
            pointer: false,
        }
    }
    fn primitive(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            primitive: true,
            pointer: false,
        }
    }
    fn pointer(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            primitive: false,
            pointer: true,
        }
    }
    fn name(&self) -> &str {
        &self.name
    }
    //fn escaped_name(&self) -> String {
    //    if self.primitive {
    //        self.name.to_string()
    //    } else {
    //        format!("`{}`", self.name)
    //    }
    //}
    fn escaped_name(&self, escape: bool) -> String {
        if self.primitive || self.pointer || !escape {
            self.name.to_string()
        } else {
            format!("`{}`", self.name)
        }
    }
}

fn type_to_string(objects: &Objects, store: &TypeStore<'_>, id: TypeId, escape: bool) -> String {
    let ctype = &store[id];
    let type_name = match ctype {
        CType::Float => TypeName::primitive("float"),
        CType::Double => TypeName::primitive("double"),
        CType::Byte => TypeName::primitive("uint8_t"), // TODO enum
        CType::UInt16 => TypeName::primitive("uint16_t"),
        CType::UInt32 => TypeName::primitive("uint32_t"),
        CType::UInt64 => TypeName::primitive("uint64_t"),
        CType::Int8 => TypeName::primitive("int8_t"),
        CType::Int16 => TypeName::primitive("int16_t"),
        CType::Int32 => TypeName::primitive("int32_t"),
        CType::Int64 => TypeName::primitive("int64_t"),

        CType::Bool => TypeName::primitive("bool"),

        CType::FName => TypeName::new("FName"),
        CType::FString => TypeName::new("FString"),
        CType::FText => TypeName::new("FText"),
        CType::MulticastInlineDelegate => TypeName::new("MulticastInlineDelegate"),
        CType::MulticastSparseDelegate => TypeName::new("MulticastSparseDelegate"),
        CType::Delegate => TypeName::new("Delegate"),

        CType::TArray(type_id) => TypeName::new(format!(
            "TArray<{}>",
            type_to_string(objects, store, *type_id, false)
        )),
        CType::TMap(k, v) => TypeName::new(format!(
            "TMap<{}, {}>",
            type_to_string(objects, store, *k, false),
            type_to_string(objects, store, *v, false)
        )),
        CType::TSet(type_id) => TypeName::new(format!(
            "TSet<{}>",
            type_to_string(objects, store, *type_id, false)
        )),
        CType::Ptr(type_id) => TypeName::pointer(format!(
            "{}*",
            type_to_string(objects, store, *type_id, escape)
        )),
        CType::TWeakObjectPtr(type_id) => TypeName::new(format!(
            "TWeakObjectPtr<{}>",
            type_to_string(objects, store, *type_id, false)
        )),
        CType::TSoftObjectPtr(type_id) => TypeName::new(format!(
            "TSoftObjectPtr<{}>",
            type_to_string(objects, store, *type_id, false)
        )),
        CType::TLazyObjectPtr(type_id) => TypeName::new(format!(
            "TLazyObjectPtr<{}>",
            type_to_string(objects, store, *type_id, false)
        )),
        CType::TScriptInterface(type_id) => TypeName::new(format!(
            "TScriptInterface<{}>",
            type_to_string(objects, store, *type_id, false)
        )),

        CType::TTuple(a, b) => {
            let a = type_to_string(objects, store, *a, false);
            let b = type_to_string(objects, store, *b, false);
            TypeName::new(format!("TTuple<{}, {}>", a, b))
        }

        CType::UEEnum(path) => TypeName::new(obj_name(objects, path)),
        CType::UEStruct(path) => TypeName::new(obj_name(objects, path)),
        CType::UEClass(path) => TypeName::new(obj_name(objects, path)),
    };
    type_name.escaped_name(escape)
}

// `TArray<Something*>`
// `Something`*
// `TArray<Something*>`*
// TArray<TArray<Something*>*>

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum DepType {
    Partial,
    Full,
}

fn get_type_dependencies<'a>(
    dependencies: &mut Vec<(DepType, TypeId)>,
    objects: &'a Objects,
    store: &mut TypeStore<'a>,
    (dep_type, id): (DepType, TypeId),
) {
    let ctype = store[id].clone();

    match (dep_type, &ctype) {
        (DepType::Full, _) => {}
        (DepType::Partial, CType::Ptr(_)) => {}
        (DepType::Partial, CType::UEEnum(_)) => {
            // add dependency on full enum because enum forward declarations aren't a thing
            dependencies.push((DepType::Full, id));
        }
        _ => return,
    }
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
        CType::Delegate => {}
        CType::TArray(type_id) => {
            let ptr_id = store.insert(CType::Ptr(type_id));
            dependencies.push((DepType::Full, ptr_id));
        }
        CType::TMap(k, v) => {
            let tuple = store.insert(CType::TTuple(k, v));
            dependencies.push((DepType::Full, tuple));
            dependencies.push((DepType::Full, k));
            dependencies.push((DepType::Full, v));
        }
        CType::TSet(k) => {
            dependencies.push((DepType::Full, k));
        }
        CType::Ptr(type_id) => {
            dependencies.push((DepType::Partial, type_id));
        }
        CType::TWeakObjectPtr(type_id) => {
            dependencies.push((DepType::Partial, type_id));
        }
        CType::TSoftObjectPtr(type_id) => {
            dependencies.push((DepType::Partial, type_id));
        }
        CType::TLazyObjectPtr(type_id) => {
            dependencies.push((DepType::Partial, type_id));
        }
        CType::TScriptInterface(type_id) => {
            dependencies.push((DepType::Partial, type_id));
        }
        CType::TTuple(a, b) => {
            dependencies.push((DepType::Full, a));
            dependencies.push((DepType::Full, b));
        }
        CType::UEEnum(_) => {}
        CType::UEStruct(path) => {
            let struct_ = &objects[path].get_struct().unwrap();
            if let Some(super_) = &struct_.super_struct {
                let super_id = store.insert(CType::UEStruct(super_));
                dependencies.push((DepType::Full, super_id));
            }
            for prop in &struct_.properties {
                let prop_id = into_ctype(objects, prop, store);
                dependencies.push((DepType::Full, prop_id));
            }
        }
        CType::UEClass(class) => {
            let class = &objects[class].get_class().unwrap();
            if let Some(super_) = &class.r#struct.super_struct {
                let super_id = store.insert(CType::UEClass(super_));
                dependencies.push((DepType::Full, super_id));
            }
            for prop in &class.r#struct.properties {
                let prop_id = into_ctype(objects, prop, store);
                dependencies.push((DepType::Full, prop_id));
            }
        }
    }
}

fn align_up(addr: usize, alignment: usize) -> usize {
    (addr + (alignment - 1)) & !alignment
}

fn get_type_size<'a>(
    objects: &'a Objects,
    store: &mut TypeStore<'a>,
    id: TypeId,
) -> (usize, usize) {
    let ctype = store[id].clone();
    match ctype {
        CType::Float => (4, 4),
        CType::Double => (8, 8),
        CType::Byte => (1, 1),
        CType::UInt16 => (2, 2),
        CType::UInt32 => (4, 4),
        CType::UInt64 => (8, 8),
        CType::Int8 => (1, 1),
        CType::Int16 => (2, 2),
        CType::Int32 => (4, 4),
        CType::Int64 => (8, 8),
        CType::Bool => (1, 1), // TODO
        CType::FName => (8, 4),
        CType::FString => (16, 8), // TODO size TArray<wchar_t>
        CType::FText => (1, 1),    // TODO
        CType::MulticastInlineDelegate => (1, 1), // TODO
        CType::MulticastSparseDelegate => (1, 1), // TODO
        CType::Delegate => (1, 1), // TODO
        CType::TArray(_) => (16, 8),
        CType::TMap(k, v) => (1, 1), // TODO
        CType::TSet(k) => (1, 1),    // TODO
        CType::Ptr(_) => (8, 8),
        CType::TWeakObjectPtr(_) => (8, 4),
        CType::TSoftObjectPtr(_) => (1, 1),
        CType::TLazyObjectPtr(_) => (1, 1),
        CType::TScriptInterface(_) => (16, 8),
        CType::TTuple(a, b) => {
            let (s_a, a_a) = get_type_size(objects, store, a);
            let (s_b, a_b) = get_type_size(objects, store, b);
            let align = a_a.max(a_b);
            let size = align_up(s_a + s_b, align);
            (size, align)
        }
        CType::UEEnum(path) => {
            // TODO unknown...
            let enum_ = &objects[path].get_enum().unwrap();
            let min = enum_.names.iter().map(|(_, v)| v).min().unwrap();
            let max = enum_.names.iter().map(|(_, v)| v).max().unwrap();
            if *min < i8::MIN as i64 || *max > u8::MAX as i64 {
                (4, 4)
            } else {
                (1, 1)
            }
        }
        CType::UEClass(path) | CType::UEStruct(path) => {
            let struct_ = &objects[path].get_struct().unwrap();
            (struct_.properties_size, struct_.min_alignment)
        }
    }
}

fn decl_ctype<'a>(
    buffer: &mut String,
    objects: &'a Objects,
    store: &mut TypeStore<'a>,
    id: TypeId,
) {
    let ctype = store[id].clone();
    let this = type_to_string(objects, store, id, true);
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
        CType::FName => {
            writeln!(buffer, r#"struct {this} {{ /* TODO */ }};"#).unwrap();
        }
        CType::FString => {
            // TODO user TArray
            writeln!(
                buffer,
                r#"struct {this} {{
    wchar_t* data;
    int32_t num;
    int32_t max;
}};"#
            )
            .unwrap();
        }
        CType::FText => {
            writeln!(buffer, r#"struct {this} {{ /* TODO */ }};"#).unwrap();
        }
        CType::MulticastInlineDelegate => {
            writeln!(buffer, r#"struct {this} {{ /* TODO */ }};"#).unwrap();
        }
        CType::MulticastSparseDelegate => {}
        CType::Delegate => {}
        CType::TArray(type_id) => {
            let ptr_id = store.insert(CType::Ptr(type_id));
            let inner = type_to_string(objects, store, ptr_id, true);
            writeln!(
                buffer,
                r#"struct {this} {{
    {inner} data;
    int32_t num;
    int32_t max;
}};"#,
            )
            .unwrap();
        }
        CType::TMap(k, v) => {
            // struct TSet<TTuple<int,FGeneratedMissionGroup>,TDefaultMapHashableKeyFuncs<int,FGeneratedMissionGroup,0>,FDefaultSetAllocator>  {
            // /* offset 0x000 */ Elements: TSparseArray<TSetElement<TTuple<int,FGeneratedMissionGroup> >,TSparseArrayAllocator<TSizedDefaultAllocator<32>,FDefaultBitArrayAllocator> >,
            // /* offset 0x038 */ Hash: TInlineAllocator<1,TSizedDefaultAllocator<32> >::ForElementType<FSetElementId>,
            // /* offset 0x048 */ HashSize: i32,

            writeln!(
                buffer,
                r#"struct {this} {{
    // TODO
}};"#,
            )
            .unwrap();
        }
        CType::TSet(type_id) => {}
        CType::Ptr(type_id) => {}
        CType::TWeakObjectPtr(type_id) => {
            writeln!(
                buffer,
                r#"struct {this} {{
    int32_t ObjectIndex;
    int32_t ObjectSerialNumber;
}};"#
            )
            .unwrap();
        }
        CType::TSoftObjectPtr(type_id) => {
            writeln!(buffer, r#"struct {this} {{ /* TODO */ }};"#).unwrap();
        }
        CType::TLazyObjectPtr(type_id) => {
            writeln!(buffer, r#"struct {this} {{ /* TODO */ }};"#).unwrap();
        }
        CType::TScriptInterface(type_id) => {
            // TODO depend on UObject* type and interface type
            writeln!(
                buffer,
                r#"struct {this} {{
    void* ObjectPointer;
    void* InterfacePointer;
}};"#
            )
            .unwrap();
        }

        CType::TTuple(a, b) => {
            let a = type_to_string(objects, store, a, true);
            let b = type_to_string(objects, store, b, true);

            writeln!(
                buffer,
                r#"struct {this} {{
    {a} a;
    {b} b;
}};"#,
            )
            .unwrap();
        }

        CType::UEEnum(path) => {
            // TODO unknown...
            let enum_ = &objects[path].get_enum().unwrap();
            let min = enum_.names.iter().map(|(_, v)| v).min().unwrap();
            let max = enum_.names.iter().map(|(_, v)| v).max().unwrap();
            let type_ = if *min < i8::MIN as i64 || *max > u8::MAX as i64 {
                "uint32_t"
            } else {
                "uint8_t"
            };
            let enum_name = obj_name(objects, path);
            let prefix = format!("{enum_name}::");
            writeln!(buffer, "enum {this} : {type_} {{").unwrap();
            if let Some((last, rest)) = enum_.names.split_last() {
                let iter = rest.iter().map(|e| (e, ",")).chain([(last, "")]);
                for ((name, value), comma) in iter {
                    let name = name.strip_prefix(&prefix).unwrap_or(&name);
                    writeln!(buffer, "    {name} = {value}{comma}",).unwrap();
                }
            }
            writeln!(buffer, "}};").unwrap();
        }
        CType::UEStruct(path) => {
            let (size, alignment) = get_type_size(objects, store, id);
            writeln!(buffer, "// size=0x{size:x} align=0x{alignment:x}").unwrap();

            let struct_ = &objects[path].get_struct().unwrap();
            let (super_, super_name, base_offset) = if let Some(super_) = &struct_.super_struct {
                let super_id = store.insert(CType::UEStruct(super_));
                let (size, _align) = get_type_size(objects, store, super_id);
                let super_name = type_to_string(objects, store, super_id, true);
                (format!("__base({super_name}, 0) "), Some(super_name), size)
            } else {
                ("".into(), None, 0)
            };

            writeln!(buffer, "class {super_}{this} {{").unwrap();
            if let Some(super_name) = super_name {
                writeln!(buffer, "    __inherited {super_name} super;").unwrap();
            }
            decl_props(buffer, objects, store, struct_, base_offset);
            writeln!(buffer, "}};").unwrap();
        }
        CType::UEClass(path) => {
            let (size, alignment) = get_type_size(objects, store, id);
            writeln!(buffer, "// size=0x{size:x} align=0x{alignment:x}").unwrap();

            let struct_ = &objects[path].get_class().unwrap().r#struct;
            let (super_, super_name, base_offset) = if let Some(super_) = &struct_.super_struct {
                let super_id = store.insert(CType::UEClass(super_));
                let (size, _align) = get_type_size(objects, store, super_id);
                let super_name = type_to_string(objects, store, super_id, true);
                (format!("__base({super_name}, 0) "), Some(super_name), size)
            } else {
                ("".into(), None, 0)
            };

            writeln!(buffer, "class {super_}{this} {{").unwrap();
            if let Some(super_name) = super_name {
                writeln!(buffer, "    __inherited {super_name} super;").unwrap();
            }
            decl_props(buffer, objects, store, struct_, base_offset);
            writeln!(buffer, "}};").unwrap();
        }
    }
}

fn decl_props<'a>(
    buffer: &mut String,
    objects: &'a Objects,
    store: &mut TypeStore<'a>,
    struct_: &'a Struct,
    mut end_last_prop: usize,
) {
    for prop in &struct_.properties {
        //if align_up(end_last_prop, alignment) != prop.offset {
        let delta = prop.offset - end_last_prop;
        if delta != 0 {
            writeln!(
                buffer,
                "    __padding char _{end_last_prop:x}[0x{delta:x}];"
            )
            .unwrap();
        }

        let ctype = into_ctype(objects, prop, store);
        let type_name = type_to_string(objects, store, ctype, true);
        writeln!(
            buffer,
            "    {} _0x{2:x}_{}; // 0x{:x}",
            type_name, prop.name, prop.offset
        )
        .unwrap();

        let (size, _alignment) = get_type_size(objects, store, ctype);
        end_last_prop = prop.offset + size;
    }
    let delta = struct_.properties_size - end_last_prop;
    if delta != 0 {
        writeln!(
            buffer,
            "    __padding char _{end_last_prop:x}[0x{delta:x}];"
        )
        .unwrap();
    }
}

trait GraphKey: Clone + Copy + PartialEq + Eq + std::hash::Hash + std::fmt::Debug {}
impl<T> GraphKey for T where T: Clone + Copy + PartialEq + Eq + std::hash::Hash + std::fmt::Debug {}
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
            println!("CYCLE {node:?}");
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
                    println!("  {node:?}");
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
                //dbg!((node, temp_visited, visited));
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
                || path.contains("GeneratedMission")
                || path.contains("CampaignManager")
                || path.contains(".Campaign")
                || path.contains(".FSDGameInstance")
        });
        println!("{header}");
        std::fs::write("header.cpp", header)?;
        Ok(())
    }
}
