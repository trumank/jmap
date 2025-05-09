use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt::Write;

use ue_reflection::{EClassCastFlags, ObjectType, Property, PropertyType, Struct};

struct Ctx<'objects, 'types> {
    header_style: HeaderStyle,

    objects: &'objects Objects,
    store: &'types mut TypeStore<'objects>,
}

type Objects = BTreeMap<String, ObjectType>;

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
            self.types.insert(id, type_);
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
        //ObjectType::Object(object) => todo!(),
        //ObjectType::Package(package) => todo!(),
        ObjectType::Enum(_) => name.into(),
        ObjectType::ScriptStruct(_script_struct) => {
            format!("F{name}")
        }
        ObjectType::Class(class) => {
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
        //ObjectType::Function(function) => todo!(),
        _ => todo!("{path} {obj:?}"),
    }
}

#[allow(unused)]
pub fn into_header(objects: &Objects, filter: impl Fn(&str, &ObjectType) -> bool) -> String {
    Ctx {
        header_style: HeaderStyle::Binja,

        objects,
        store: &mut TypeStore::default(),
    }
    .generate(filter)
}

#[derive(Debug, Clone, Copy)]
enum HeaderStyle {
    Binja,
    C,
}
impl HeaderStyle {
    fn class_name(&self) -> &'static str {
        match self {
            HeaderStyle::Binja => "class",
            HeaderStyle::C => "struct",
        }
    }
    fn format_template(
        &self,
        name: &str,
        params: impl IntoIterator<Item = impl AsRef<str>>,
    ) -> String {
        let (c_open, c_sep, c_close) = match self {
            HeaderStyle::Binja => ("<", ", ", ">"),
            HeaderStyle::C => ("_", "_", "_"),
        };

        let mut buffer = String::new();
        buffer.push_str(name);

        buffer.push_str(c_open);

        let mut iter = params.into_iter();
        if let Some(first) = iter.next() {
            buffer.push_str(first.as_ref());
        }
        for next in iter {
            buffer.push_str(c_sep);
            buffer.push_str(next.as_ref());
        }

        buffer.push_str(c_close);

        buffer
    }
}

impl<'objects> Ctx<'objects, '_> {
    fn prop_ctype(&mut self, prop: &'objects Property) -> TypeId {
        let new_type = match &prop.r#type {
            PropertyType::Struct { r#struct } => CType::UEStruct(r#struct),
            PropertyType::Str => CType::FString,
            PropertyType::Name => CType::FName,
            PropertyType::Text => CType::FText,
            PropertyType::FieldPath => CType::FFieldPath,
            PropertyType::MulticastInlineDelegate => CType::MulticastInlineDelegate, // TODO
            PropertyType::MulticastSparseDelegate => CType::MulticastSparseDelegate, // TODO
            PropertyType::Delegate => CType::Delegate,
            PropertyType::Bool {
                field_size,
                byte_offset,
                byte_mask,
                field_mask,
            } => {
                let inner = match field_size {
                    1 => CType::UInt8,
                    //2 => CType::UInt16,
                    //4 => CType::UInt32,
                    //8 => CType::UInt64,
                    _ => todo!("handle bitfield field_size={field_size}"),
                };
                let inner = self.store.insert(inner);
                let index = get_bitfield_bit_index(*byte_offset, *byte_mask);
                CType::Bool(inner, index)
            }
            PropertyType::Array { inner } => CType::TArray(self.prop_ctype(inner)),
            PropertyType::Enum { container, r#enum } => {
                CType::UEEnum(r#enum.as_ref().expect("TODO unknown enum name"))
            }
            PropertyType::Map {
                key_prop,
                value_prop,
            } => {
                let key = self.prop_ctype(key_prop);
                let value = self.prop_ctype(value_prop);
                CType::TMap(key, value)
            }
            PropertyType::Set { key_prop } => CType::TSet(self.prop_ctype(key_prop)),
            PropertyType::Float => CType::Float,
            PropertyType::Double => CType::Double,
            PropertyType::Byte { r#enum } => CType::UInt8,
            PropertyType::UInt16 => CType::UInt16,
            PropertyType::UInt32 => CType::UInt32,
            PropertyType::UInt64 => CType::UInt64,
            PropertyType::Int8 => CType::Int8,
            PropertyType::Int16 => CType::Int16,
            PropertyType::Int => CType::Int32,
            PropertyType::Int64 => CType::Int64,
            PropertyType::Object { class } => {
                let class = CType::UEClass(class.as_deref().expect("expected class name"));
                CType::Ptr(self.store.insert(class))
            }
            PropertyType::WeakObject { class } => {
                let class = CType::UEClass(class);
                CType::TWeakObjectPtr(self.store.insert(class))
            }
            PropertyType::SoftObject { class } => {
                let class = CType::UEClass(class);
                CType::TSoftObjectPtr(self.store.insert(class))
            }
            PropertyType::LazyObject { class } => {
                let class = CType::UEClass(class);
                CType::TLazyObjectPtr(self.store.insert(class))
            }
            PropertyType::Interface { class } => {
                let class = CType::UEClass(class);
                CType::TScriptInterface(self.store.insert(class))
            }
            PropertyType::Optional { inner } => todo!(),
        };
        let id = self.store.insert(new_type);
        if prop.array_dim == 1 {
            id
        } else {
            self.store.insert(CType::Array(id, prop.array_dim))
        }
    }

    fn type_to_string(&mut self, id: TypeId, escape: bool, in_template: bool) -> String {
        let escape_inner = match self.header_style {
            HeaderStyle::Binja => escape,
            HeaderStyle::C => false,
        };
        let ctype = self.store[id];
        let type_name = match ctype {
            CType::Float => TypeName::primitive("float"),
            CType::Double => TypeName::primitive("double"),
            CType::UInt8 => TypeName::primitive("uint8_t"), // TODO enum
            CType::UInt16 => TypeName::primitive("uint16_t"),
            CType::UInt32 => TypeName::primitive("uint32_t"),
            CType::UInt64 => TypeName::primitive("uint64_t"),
            CType::Int8 => TypeName::primitive("int8_t"),
            CType::Int16 => TypeName::primitive("int16_t"),
            CType::Int32 => TypeName::primitive("int32_t"),
            CType::Int64 => TypeName::primitive("int64_t"),

            CType::WChar => TypeName::primitive("wchar_t"),

            CType::Bool(type_id, _) => TypeName::new(self.type_to_string(type_id, false, false)),

            CType::FName => TypeName::new("FName"),
            CType::FString => TypeName::new("FString"),
            CType::FText => TypeName::new("FText"),
            CType::FFieldPath => TypeName::new("FFieldPath"),
            CType::MulticastInlineDelegate => TypeName::new("MulticastInlineDelegate"),
            CType::MulticastSparseDelegate => TypeName::new("MulticastSparseDelegate"),
            CType::Delegate => TypeName::new("Delegate"),

            CType::TArray(type_id) => {
                let inner = self.type_to_string(type_id, false, true);
                TypeName::new(self.header_style.format_template("TArray", [inner]))
            }
            CType::TMap(k, v) => {
                let k = self.type_to_string(k, false, true);
                let v = self.type_to_string(v, false, true);
                TypeName::new(self.header_style.format_template("TMap", [k, v]))
            }
            CType::TSet(type_id) => {
                let inner = self.type_to_string(type_id, false, true);
                TypeName::new(self.header_style.format_template("TSet", [inner]))
            }
            CType::Ptr(type_id) => TypeName::pointer(format!(
                "{}{}",
                self.type_to_string(type_id, escape_inner, in_template),
                if in_template { "P" } else { "*" }
            )),
            CType::TWeakObjectPtr(type_id) => {
                let inner = self.type_to_string(type_id, false, true);
                TypeName::new(self.header_style.format_template("TWeakObjectPtr", [inner]))
            }
            CType::TSoftObjectPtr(type_id) => {
                let inner = self.type_to_string(type_id, false, true);
                TypeName::new(self.header_style.format_template("TSoftObjectPtr", [inner]))
            }
            CType::TLazyObjectPtr(type_id) => {
                let inner = self.type_to_string(type_id, false, true);
                TypeName::new(self.header_style.format_template("TLazyObjectPtr", [inner]))
            }
            CType::TScriptInterface(type_id) => {
                let inner = self.type_to_string(type_id, false, true);
                TypeName::new(
                    self.header_style
                        .format_template("TScriptInterface", [inner]),
                )
            }
            CType::TTuple(a, b) => {
                let a = self.type_to_string(a, false, true);
                let b = self.type_to_string(b, false, true);
                TypeName::new(self.header_style.format_template("TTuple", [a, b]))
            }

            CType::Array(type_id, _size) => {
                TypeName::new(self.type_to_string(type_id, false, in_template))
            } // handle size at struct member, not here

            CType::UEEnum(path) => TypeName::new(obj_name(self.objects, path)),
            CType::UEStruct(path) => TypeName::new(obj_name(self.objects, path)),
            CType::UEClass(path) => TypeName::new(obj_name(self.objects, path)),
        };
        type_name.escaped_name(escape_inner)
    }

    fn get_type_dependencies(
        &mut self,
        dependencies: &mut Vec<(DepType, TypeId)>,
        (dep_type, id): (DepType, TypeId),
    ) {
        let ctype = self.store[id];

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
            CType::UInt8 => {}
            CType::UInt16 => {}
            CType::UInt32 => {}
            CType::UInt64 => {}
            CType::Int8 => {}
            CType::Int16 => {}
            CType::Int32 => {}
            CType::Int64 => {}
            CType::WChar => {}
            CType::Bool(type_id, _field_mask) => {
                dependencies.push((DepType::Full, type_id));
            }
            CType::FName => {}
            CType::FString => {
                dependencies.push((DepType::Full, type_fstring_data(self.store)));
            }
            CType::FText => {}
            CType::FFieldPath => {}
            CType::MulticastInlineDelegate => {}
            CType::MulticastSparseDelegate => {}
            CType::Delegate => {}
            CType::TArray(type_id) => {
                let ptr_id = self.store.insert(CType::Ptr(type_id));
                dependencies.push((DepType::Full, ptr_id));
            }
            CType::TMap(k, v) => {
                let tuple = self.store.insert(CType::TTuple(k, v));
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
            CType::Array(type_id, _size) => {
                dependencies.push((DepType::Full, type_id));
            }
            CType::UEEnum(_) => {}
            CType::UEStruct(path) => {
                let struct_ = &self.objects[path].get_struct().unwrap();
                if let Some(super_) = &struct_.super_struct {
                    let super_id = self.store.insert(CType::UEStruct(super_));
                    dependencies.push((DepType::Full, super_id));
                }
                for prop in &struct_.properties {
                    let prop_id = self.prop_ctype(prop);
                    dependencies.push((DepType::Full, prop_id));
                }
            }
            CType::UEClass(class) => {
                let class = &self.objects[class].get_class().unwrap();
                if let Some(super_) = &class.r#struct.super_struct {
                    let super_id = self.store.insert(CType::UEClass(super_));
                    dependencies.push((DepType::Full, super_id));
                }
                for prop in &class.r#struct.properties {
                    let prop_id = self.prop_ctype(prop);
                    dependencies.push((DepType::Full, prop_id));
                }
            }
        }
    }

    fn get_type_size(&mut self, id: TypeId) -> (usize, usize) {
        let ctype = self.store[id];
        match ctype {
            CType::Float => (4, 4),
            CType::Double => (8, 8),
            CType::UInt8 => (1, 1),
            CType::UInt16 => (2, 2),
            CType::UInt32 => (4, 4),
            CType::UInt64 => (8, 8),
            CType::Int8 => (1, 1),
            CType::Int16 => (2, 2),
            CType::Int32 => (4, 4),
            CType::Int64 => (8, 8),
            CType::WChar => (2, 2),
            CType::Bool(type_id, _field_mask) => self.get_type_size(type_id),
            CType::FName => (8, 4),
            CType::FString => (16, 8),    // TODO size TArray<wchar_t>
            CType::FText => (1, 1),       // TODO
            CType::FFieldPath => (32, 8), // TODO
            CType::MulticastInlineDelegate => (16, 8), // TODO
            CType::MulticastSparseDelegate => (1, 1), // TODO
            CType::Delegate => (1, 1),    // TODO
            CType::TArray(_) => (16, 8),
            CType::TMap(k, v) => (1, 1), // TODO
            CType::TSet(k) => (1, 1),    // TODO
            CType::Ptr(_) => (8, 8),
            CType::TWeakObjectPtr(_) => (8, 4),
            CType::TSoftObjectPtr(_) => (1, 1),
            CType::TLazyObjectPtr(_) => (1, 1),
            CType::TScriptInterface(_) => (16, 8),
            CType::TTuple(a, b) => {
                let (s_a, a_a) = self.get_type_size(a);
                let (s_b, a_b) = self.get_type_size(b);
                let align = a_a.max(a_b);
                let size = align_up(s_a + s_b, align);
                (size, align)
            }
            CType::Array(type_id, size) => {
                let (inner_size, alignment) = self.get_type_size(type_id);
                (size * inner_size, alignment)
            }
            CType::UEEnum(path) => {
                // TODO unknown...
                let enum_ = &self.objects[path].get_enum().unwrap();
                let min = enum_.names.iter().map(|(_, v)| v).min().unwrap();
                let max = enum_.names.iter().map(|(_, v)| v).max().unwrap();
                if *min < i8::MIN as i64 || *max > u8::MAX as i64 {
                    (4, 4)
                } else {
                    (1, 1)
                }
            }
            CType::UEClass(path) | CType::UEStruct(path) => {
                let struct_ = &self.objects[path].get_struct().unwrap();
                (struct_.properties_size, struct_.min_alignment)
            }
        }
    }

    fn decl_ctype(&mut self, buffer: &mut String, id: TypeId) {
        let ctype = self.store[id];
        let this = self.type_to_string(id, true, false);
        match ctype {
            CType::Float => {}
            CType::Double => {}
            CType::UInt8 => {}
            CType::UInt16 => {}
            CType::UInt32 => {}
            CType::UInt64 => {}
            CType::Int8 => {}
            CType::Int16 => {}
            CType::Int32 => {}
            CType::Int64 => {}
            CType::WChar => {}
            CType::Bool(_, _) => {}
            CType::FName => {
                writeln!(
                    buffer,
                    r#"struct {this} {{
    uint32_t ComparisonIndex;
    uint32_t Number;
}};"#
                )
                .unwrap();
            }
            CType::FString => {
                let data = type_fstring_data(self.store);
                let data_name = self.type_to_string(data, true, false);
                writeln!(
                    buffer,
                    r#"struct {this} {{
    {data_name} data;
}};"#
                )
                .unwrap();
            }
            CType::FText => {
                writeln!(buffer, r#"struct {this} {{ /* TODO */ }};"#).unwrap();
            }
            CType::FFieldPath => {
                // TODO
                writeln!(
                    buffer,
                    r#"struct {this} {{
    void* ResolvedField;
    void* ResolvedOwner;
    void* PathData;
    int32_t PathNum;
    int32_t PathMax;
}};"#
                )
                .unwrap();
            }
            CType::MulticastInlineDelegate => {
                // TODO user TArray
                writeln!(
                    buffer,
                    r#"struct {this} {{
    void* data;
    int32_t num;
    int32_t max;
}};"#
                )
                .unwrap();
            }
            CType::MulticastSparseDelegate => {
                writeln!(buffer, r#"struct {this} {{ /* TODO */ }};"#).unwrap();
            }
            CType::Delegate => {
                writeln!(buffer, r#"struct {this} {{ /* TODO */ }};"#).unwrap();
            }
            CType::TArray(type_id) => {
                let ptr_id = self.store.insert(CType::Ptr(type_id));
                let inner = self.type_to_string(ptr_id, true, false);
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
            CType::TSet(k) => writeln!(
                buffer,
                r#"struct {this} {{
    // TODO
}};"#,
            )
            .unwrap(),
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
                let a = self.type_to_string(a, true, false);
                let b = self.type_to_string(b, true, false);

                writeln!(
                    buffer,
                    r#"struct {this} {{
    {a} a;
    {b} b;
}};"#,
                )
                .unwrap();
            }

            CType::Array(_, _) => {}

            CType::UEEnum(path) => {
                // TODO unknown...
                let enum_ = &self.objects[path].get_enum().unwrap();
                let min = enum_.names.iter().map(|(_, v)| v).min().unwrap();
                let max = enum_.names.iter().map(|(_, v)| v).max().unwrap();
                let type_ = if *min < i8::MIN as i64 || *max > u8::MAX as i64 {
                    "uint32_t"
                } else {
                    "uint8_t"
                };
                let enum_name = obj_name(self.objects, path);
                let prefix = format!("{enum_name}::");
                writeln!(buffer, "enum {this} : {type_} {{").unwrap();
                if let Some((last, rest)) = enum_.names.split_last() {
                    let iter = rest.iter().map(|e| (e, ",")).chain([(last, "")]);
                    for ((name, value), comma) in iter {
                        //let name = name.strip_prefix(&prefix).unwrap_or(&name);
                        let name = match self.header_style {
                            HeaderStyle::Binja => {
                                format!("`{name}`")
                            }
                            HeaderStyle::C => name.replace(":", "_"),
                        };
                        writeln!(buffer, "    {name} = {value}{comma}",).unwrap();
                    }
                }
                writeln!(buffer, "}};").unwrap();
            }
            CType::UEStruct(path) | CType::UEClass(path) => {
                let class_or_struct = match (self.header_style, ctype) {
                    (HeaderStyle::Binja, CType::UEStruct(_)) => "struct",
                    (HeaderStyle::Binja, CType::UEClass(_)) => "class",
                    (HeaderStyle::C, _) => self.header_style.class_name(),
                    _ => unreachable!(),
                };

                let (size, alignment) = self.get_type_size(id);
                writeln!(buffer, "// size=0x{size:x} align=0x{alignment:x}").unwrap();

                let struct_ = &self.objects[path].get_struct().unwrap();
                let (super_, super_name, base_offset) = if let Some(super_) = &struct_.super_struct
                {
                    let super_id = self.store.insert(CType::UEClass(super_));
                    let (size, _align) = self.get_type_size(super_id);
                    let super_name = self.type_to_string(super_id, true, false);
                    let base = match self.header_style {
                        HeaderStyle::Binja => format!("__base({super_name}, 0) "),
                        HeaderStyle::C => "".into(),
                    };
                    (base, Some(super_name), size)
                } else {
                    ("".into(), None, 0)
                };

                let align_attribute = match self.header_style {
                    HeaderStyle::Binja => "".to_string(),
                    HeaderStyle::C => format!("__attribute__((aligned({alignment}))) "),
                };

                writeln!(
                    buffer,
                    "{class_or_struct} {align_attribute}{super_}{this} {{"
                )
                .unwrap();
                //if let Some(super_name) = super_name {
                //    let inherited = match self.header_style {
                //        HeaderStyle::Binja => "__inherited ",
                //        HeaderStyle::C => "",
                //    };
                //    writeln!(buffer, "    {inherited}{super_name} super;").unwrap();
                //}

                let mut offset = match self.header_style {
                    HeaderStyle::Binja => {
                        let mut parent = *struct_;
                        let mut parents = vec![];
                        while let Some(next) = &parent.super_struct {
                            parent = self.objects[next].get_struct().unwrap();
                            parents.push((next, parent));
                        }

                        let mut offset = 0;

                        for (path, parent) in parents.iter().rev() {
                            self.decl_props(
                                buffer,
                                parent,
                                &mut offset,
                                Some(&obj_name(self.objects, path)),
                            );
                        }

                        offset
                    }
                    HeaderStyle::C => base_offset,
                };

                self.decl_props(buffer, struct_, &mut offset, None);
                writeln!(buffer, "}};").unwrap();

                match self.header_style {
                    HeaderStyle::Binja => {}
                    HeaderStyle::C => {
                        writeln!(buffer, "static_assert(sizeof({this}) == 0x{size:x}, \"{this} has incorrect size\");").unwrap();
                        writeln!(buffer, "static_assert(alignof({this}) == 0x{alignment:x}, \"{this} has incorrect alignment\");").unwrap();
                    }
                }
            }
        }
    }

    fn decl_props(
        &mut self,
        buffer: &mut String,
        struct_: &'objects Struct,
        end_last_prop: &mut usize,
        inherited_from: Option<&str>,
    ) {
        let header_style = self.header_style;
        let pad = |buffer: &mut String, expected: usize, at: usize| {
            let delta = expected.saturating_sub(at);
            if delta != 0 {
                let pad = match header_style {
                    HeaderStyle::Binja => "__padding ",
                    HeaderStyle::C => "/* pad */ ",
                };
                writeln!(buffer, "    {pad}char _{at:x}[0x{delta:x}];").unwrap();
            }
        };
        for prop in &struct_.properties {
            pad(buffer, prop.offset, *end_last_prop);

            let ctype = self.prop_ctype(prop);
            let type_name = self.type_to_string(ctype, true, false);

            let prop_offset = prop.offset;

            let prop_name = format!("_0x{prop_offset:x}_{}", prop.name);
            let prop_name = match (inherited_from, self.header_style) {
                (None, _) => prop_name,
                (Some(parent), HeaderStyle::Binja) => format!("`{parent}::{prop_name}`"),
                (_, HeaderStyle::C) => prop_name,
            };

            let postfix = match self.store[ctype] {
                CType::Array(_, size) => format!("[{size}]"), // TODO multi-dimension?
                CType::Bool(_, _) => ":1".into(),
                _ => "".into(),
            };

            let inherited = match inherited_from {
                Some(_) => "__inherited ",
                None => "",
            };

            writeln!(
                buffer,
                "    {inherited}{type_name} {prop_name}{postfix}; // 0x{prop_offset:x}",
            )
            .unwrap();

            let (size, _alignment) = self.get_type_size(ctype);
            *end_last_prop = prop.offset + size;
        }
        // do not add trailing padding if these are inherited props
        if inherited_from.is_none() {
            pad(buffer, struct_.properties_size, *end_last_prop);
        }
    }

    fn generate(&mut self, filter: impl Fn(&str, &ObjectType) -> bool) -> String {
        let mut buffer = String::new();

        match self.header_style {
            HeaderStyle::Binja => {}
            HeaderStyle::C => {
                writeln!(&mut buffer, "#include <stdint.h>\n").unwrap();
            }
        }

        let mut to_visit = HashSet::new();
        let mut dep_graph = HashMap::new();
        // get dependencies of initial top level classes
        for (path, obj) in self.objects {
            if filter(path, obj) && obj.get_class().is_some() {
                let mut dependencies = vec![];
                let class_id = self.store.insert(CType::UEClass(path));
                let type_ = (DepType::Full, class_id);
                self.get_type_dependencies(&mut dependencies, type_);
                dep_graph.insert(type_, dependencies.clone());

                // in the rare circumstance the current item was already added to the to_visit list
                to_visit.remove(&type_);

                for dep in dependencies {
                    if !dep_graph.contains_key(&dep) {
                        to_visit.insert(dep);
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
        println!(" --- ERM --- ");

        fn pop<T: Clone + Eq + std::hash::Hash>(set: &mut HashSet<T>) -> Option<T> {
            set.iter().next().cloned().inspect(|item| {
                set.remove(item);
            })
        }

        // get dependencies of dependencies
        while let Some(next) = pop(&mut to_visit) {
            let mut dependencies = vec![];
            if !dep_graph.contains_key(&next) {
                self.get_type_dependencies(&mut dependencies, next);
            }
            assert!(dep_graph.insert(next, dependencies.clone()).is_none());
            for dep in dependencies {
                if !dep_graph.contains_key(&dep) {
                    to_visit.insert(dep);
                    //if dep.0 == DepType::Partial {
                    //    let full = (DepType::Full, dep.1);
                    //    if !dep_graph.contains_key(&full) {
                    //        to_visit.push(full);
                    //    }
                    //}
                }
            }
        }

        //dbg!(&dep_graph);
        //dbg!(&type_store.types);

        // debug print graph
        //for (owner, dependencies) in &dep_graph {
        //    println!("{:?} {}", owner, self.type_to_string(owner.1, false, false));
        //    for dep in dependencies {
        //        println!("  {:?} {}", dep, self.type_to_string(dep.1, false, false));
        //    }
        //}

        // forward declarations
        for (dep_type, type_id) in dep_graph.keys() {
            if *dep_type == DepType::Partial {
                let type_ = self.store[*type_id];
                let this = self.type_to_string(*type_id, true, false);
                match type_ {
                    //CType::Ptr(type_id) => todo!(),
                    CType::FName
                    | CType::FString
                    | CType::FText
                    | CType::FFieldPath
                    | CType::TArray(_)
                    | CType::TMap(_, _)
                    | CType::TSet(_)
                    | CType::TWeakObjectPtr(_)
                    | CType::TSoftObjectPtr(_)
                    | CType::TScriptInterface(_)
                    | CType::TTuple(_, _)
                    | CType::UEStruct(_) => {
                        writeln!(&mut buffer, "struct {this};").unwrap();
                    }
                    CType::UEClass(_) => {
                        let class_or_struct = self.header_style.class_name();
                        writeln!(&mut buffer, "{class_or_struct} {this};").unwrap();
                    }
                    _ => {}
                }
            }
        }

        let sorted = topological_sort(&dep_graph).unwrap();
        //dbg!(&sorted);

        // full declarations
        for (dep_type, type_id) in &sorted {
            if *dep_type == DepType::Full {
                self.decl_ctype(&mut buffer, *type_id);
            }
        }

        buffer
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct TypeId(usize);
impl std::fmt::Debug for TypeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "TypeId({})", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum CType<'a> {
    Float,
    Double,
    UInt8,
    UInt16,
    UInt32,
    UInt64,
    Int8,
    Int16,
    Int32,
    Int64,

    WChar,

    Bool(TypeId, usize), // TODO bitfield

    FName,
    FString,
    FText,
    FFieldPath,
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

    Array(TypeId, usize),

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
// `TArray<Something*>`
// `Something`*
// `TArray<Something*>`*
// TArray<TArray<Something*>*>

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum DepType {
    Partial,
    Full,
}

fn align_up(addr: usize, alignment: usize) -> usize {
    (addr + (alignment - 1)) & !alignment
}

fn get_bitfield_bit_index(byte_offset: u8, byte_mask: u8) -> usize {
    byte_offset as usize + 8 - byte_mask.leading_zeros() as usize
}

fn type_fstring_data(store: &mut TypeStore<'_>) -> TypeId {
    let t_wchar = store.insert(CType::WChar);
    store.insert(CType::TArray(t_wchar))
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
        if !visited.contains(node)
            && !dfs(*node, graph, &mut visited, &mut temp_visited, &mut result)
        {
            return None; // Graph has a cycle
        }
    }

    Some(result)
}

#[cfg(test)]
mod test {
    use super::*;

    use anyhow::Result;

    #[test]
    fn test_bitfield() {
        assert_eq!(0x0, get_bitfield_bit_index(0, 0b0000_0001));
        assert_eq!(0x1, get_bitfield_bit_index(0, 0b0000_0010));
        assert_eq!(0x2, get_bitfield_bit_index(0, 0b0000_0100));
        assert_eq!(0x3, get_bitfield_bit_index(0, 0b0000_1000));
        assert_eq!(0x4, get_bitfield_bit_index(0, 0b0001_0000));
        assert_eq!(0x5, get_bitfield_bit_index(0, 0b0010_0000));
        assert_eq!(0x6, get_bitfield_bit_index(0, 0b0100_0000));
        assert_eq!(0x7, get_bitfield_bit_index(0, 0b1000_0000));

        assert_eq!(0x8, get_bitfield_bit_index(1, 0b0000_0001));
        assert_eq!(0x9, get_bitfield_bit_index(1, 0b0000_0010));
        assert_eq!(0xa, get_bitfield_bit_index(1, 0b0000_0100));
        assert_eq!(0xb, get_bitfield_bit_index(1, 0b0000_1000));
        assert_eq!(0xc, get_bitfield_bit_index(1, 0b0001_0000));
        assert_eq!(0xd, get_bitfield_bit_index(1, 0b0010_0000));
        assert_eq!(0xe, get_bitfield_bit_index(1, 0b0100_0000));
        assert_eq!(0xf, get_bitfield_bit_index(1, 0b1000_0000));
    }

    #[test]
    fn test_into_header() -> Result<()> {
        let objects: Objects = serde_json::from_slice(&std::fs::read("../fsd.json")?)?;
        let header = into_header(&objects, |path, obj| {
            //path.contains("MissionGenerationManager")
            //|| path.contains("GeneratedMission")
            //|| path.contains("CampaignManager")
            //|| path.contains(".Campaign")
            //path.contains(".FSDSaveGame")
            path.contains(".CameraComponent")
            //true
        });
        //println!("{header}");
        std::fs::write("header.h", header)?;
        Ok(())
    }
}
