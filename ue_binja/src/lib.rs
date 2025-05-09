use std::collections::{HashMap, HashSet};
use std::fmt::Write;
use std::num::NonZero;

use anyhow::Result;
use binaryninja::architecture::CoreArchitecture;
use binaryninja::binary_view::BinaryView;
use binaryninja::rc::Ref;
use binaryninja::symbol::{Symbol, SymbolType};
use binaryninja::types::{
    BaseStructure, NamedTypeReference, NamedTypeReferenceClass, StructureBuilder,
};
use binaryninja::{
    binary_view::BinaryViewExt,
    command::{self, Command},
    logger::Logger,
    types::{MemberAccess, MemberScope, Structure, Type},
};
use log::{error, info};

use ue_reflection::{
    Class, EClassCastFlags, ObjectType, Property, PropertyType, ReflectionData, Struct,
};

struct ImportCommand {}

impl Command for ImportCommand {
    fn action(&self, bv: &binaryninja::binary_view::BinaryView) {
        info!("do the stuff");

        let ref_data = match load() {
            Ok(d) => d,
            Err(e) => {
                error!("failed to load objects: {e}");
                return;
            }
        };

        let action = bv.file().begin_undo_actions(false);

        info!("loaded {} objects", ref_data.objects.len());

        into_header(&ref_data, bv, |_path, _obj| true);

        bv.file().commit_undo_actions(action);
    }

    fn valid(&self, _view: &binaryninja::binary_view::BinaryView) -> bool {
        true
    }
}

fn load() -> Result<ReflectionData> {
    let path = "/home/truman/projects/ue/meatloaf/fsd.json";
    Ok(serde_json::from_reader(std::io::BufReader::new(
        std::fs::File::open(path)?,
    ))?)
}

#[no_mangle]
pub extern "C" fn CorePluginInit() -> bool {
    Logger::new("ue_binja").init();

    info!("ue_binja loaded");

    command::register_command(
        "ue_binja - import reflection data",
        "Import Unreal Engine reflection data from meatloaf",
        ImportCommand {},
    );

    true
}

struct Ctx<'ref_data, 'types, 'bv> {
    header_style: HeaderStyle,

    bv: &'bv BinaryView,

    ref_data: &'ref_data ReflectionData,
    store: &'types mut TypeStore<'ref_data>,
}

struct TypeStore<'a> {
    next: NonZero<usize>,
    types: HashMap<TypeId, CType<'a>>,
    types_reverse: HashMap<CType<'a>, TypeId>,
}
impl Default for TypeStore<'_> {
    fn default() -> Self {
        Self {
            next: 1.try_into().unwrap(),
            types: Default::default(),
            types_reverse: Default::default(),
        }
    }
}

impl<'a> TypeStore<'a> {
    fn insert(&mut self, type_: CType<'a>) -> TypeId {
        if let Some(existing) = self.types_reverse.get(&type_) {
            *existing
        } else {
            let id = TypeId(self.next);
            self.types.insert(id, type_);
            self.types_reverse.insert(type_, id);
            self.next = self.next.checked_add(1).unwrap();
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

fn obj_name(ref_data: &ReflectionData, path: &str) -> String {
    let obj = &ref_data.objects[path];
    let name = path.rsplit(['/', '.', ':']).next().unwrap();
    match obj {
        ObjectType::Object(_) => name.to_string(),
        ObjectType::Package(_) => name.to_string(),
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
        ObjectType::Function(_) => name.to_string(),
    }
}

#[allow(unused)]
pub fn into_header(
    ref_data: &ReflectionData,
    bv: &BinaryView,
    filter: impl Fn(&str, &ObjectType) -> bool,
) {
    Ctx {
        header_style: HeaderStyle::Binja,

        bv,

        ref_data,
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

impl<'ref_data> Ctx<'ref_data, '_, '_> {
    fn prop_ctype(&mut self, prop: &'ref_data Property) -> TypeId {
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
                field_mask: _,
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
            PropertyType::Enum {
                container: _,
                r#enum,
            } => CType::UEEnum(r#enum.as_ref().expect("TODO unknown enum name")),
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
            PropertyType::Byte { r#enum: _ } => CType::UInt8,
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
            PropertyType::Optional { inner: _ } => todo!(),
        };
        let id = self.store.insert(new_type);
        if prop.array_dim == 1 {
            id
        } else {
            self.store.insert(CType::Array(id, prop.array_dim))
        }
    }

    fn type_to_string(&mut self, id: TypeId, escape: bool) -> String {
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

            CType::Bool(type_id, _) => TypeName::new(self.type_to_string(type_id, false)),

            CType::FName => TypeName::new("FName"),
            CType::FString => TypeName::new("FString"),
            CType::FText => TypeName::new("FText"),
            CType::FFieldPath => TypeName::new("FFieldPath"),
            CType::MulticastInlineDelegate => TypeName::new("MulticastInlineDelegate"),
            CType::MulticastSparseDelegate => TypeName::new("MulticastSparseDelegate"),
            CType::Delegate => TypeName::new("Delegate"),

            CType::TArray(type_id) => {
                let inner = self.type_to_string(type_id, false);
                TypeName::new(self.header_style.format_template("TArray", [inner]))
            }
            CType::TMap(k, v) => {
                let k = self.type_to_string(k, false);
                let v = self.type_to_string(v, false);
                TypeName::new(self.header_style.format_template("TMap", [k, v]))
            }
            CType::TSet(type_id) => {
                let inner = self.type_to_string(type_id, false);
                TypeName::new(self.header_style.format_template("TSet", [inner]))
            }
            CType::Ptr(type_id) => {
                TypeName::pointer(format!("{}*", self.type_to_string(type_id, escape_inner),))
            }
            CType::TWeakObjectPtr(type_id) => {
                let inner = self.type_to_string(type_id, false);
                TypeName::new(self.header_style.format_template("TWeakObjectPtr", [inner]))
            }
            CType::TSoftObjectPtr(type_id) => {
                let inner = self.type_to_string(type_id, false);
                TypeName::new(self.header_style.format_template("TSoftObjectPtr", [inner]))
            }
            CType::TLazyObjectPtr(type_id) => {
                let inner = self.type_to_string(type_id, false);
                TypeName::new(self.header_style.format_template("TLazyObjectPtr", [inner]))
            }
            CType::TScriptInterface(type_id) => {
                let inner = self.type_to_string(type_id, false);
                TypeName::new(
                    self.header_style
                        .format_template("TScriptInterface", [inner]),
                )
            }
            CType::TTuple(a, b) => {
                let a = self.type_to_string(a, false);
                let b = self.type_to_string(b, false);
                TypeName::new(self.header_style.format_template("TTuple", [a, b]))
            }

            CType::Array(type_id, _size) => TypeName::new(self.type_to_string(type_id, false)), // handle size at struct member, not here

            CType::UEEnum(path) => TypeName::new(obj_name(self.ref_data, path)),
            CType::UEStruct(path) => TypeName::new(obj_name(self.ref_data, path)),
            CType::UEClass(path) => TypeName::new(obj_name(self.ref_data, path)),
        };
        type_name.escaped_name(escape_inner)
    }

    fn bn_type(&mut self, id: TypeId) -> Ref<Type> {
        let ctype = self.store[id];
        let struct_ = |name: &str| {
            Type::named_type(&NamedTypeReference::new(
                NamedTypeReferenceClass::StructNamedTypeClass,
                name,
            ))
        };
        match ctype {
            CType::Float => Type::float(4),
            CType::Double => Type::float(8),
            CType::UInt8 => Type::int(1, false), // TODO enum
            CType::UInt16 => Type::int(2, false),
            CType::UInt32 => Type::int(4, false),
            CType::UInt64 => Type::int(8, false),
            CType::Int8 => Type::int(1, true),
            CType::Int16 => Type::int(2, true),
            CType::Int32 => Type::int(4, true),
            CType::Int64 => Type::int(8, true),

            CType::WChar => Type::named_int(2, false, "wchar_t"),

            CType::Bool(type_id, _) => self.bn_type(type_id),

            CType::FName => struct_("FName"),
            CType::FString => struct_("FString"),
            CType::FText => struct_("FText"),
            CType::FFieldPath => struct_("FFieldPath"),
            CType::MulticastInlineDelegate => struct_("MulticastInlineDelegate"),
            CType::MulticastSparseDelegate => struct_("MulticastSparseDelegate"),
            CType::Delegate => struct_("Delegate"),

            CType::TArray(type_id) => {
                let inner = self.type_to_string(type_id, false);
                struct_(&self.header_style.format_template("TArray", [inner]))
            }
            CType::TMap(k, v) => {
                let k = self.type_to_string(k, false);
                let v = self.type_to_string(v, false);
                struct_(&self.header_style.format_template("TMap", [k, v]))
            }
            CType::TSet(type_id) => {
                let inner = self.type_to_string(type_id, false);
                struct_(&self.header_style.format_template("TSet", [inner]))
            }
            CType::Ptr(type_id) => Type::pointer(
                &CoreArchitecture::by_name("x86_64").unwrap(),
                &self.bn_type(type_id),
            ),
            CType::TWeakObjectPtr(type_id) => {
                let inner = self.type_to_string(type_id, false);
                struct_(&self.header_style.format_template("TWeakObjectPtr", [inner]))
            }
            CType::TSoftObjectPtr(type_id) => {
                let inner = self.type_to_string(type_id, false);
                struct_(&self.header_style.format_template("TSoftObjectPtr", [inner]))
            }
            CType::TLazyObjectPtr(type_id) => {
                let inner = self.type_to_string(type_id, false);
                struct_(&self.header_style.format_template("TLazyObjectPtr", [inner]))
            }
            CType::TScriptInterface(type_id) => {
                let inner = self.type_to_string(type_id, false);
                struct_(
                    &self
                        .header_style
                        .format_template("TScriptInterface", [inner]),
                )
            }
            CType::TTuple(a, b) => {
                let a = self.type_to_string(a, false);
                let b = self.type_to_string(b, false);
                struct_(&self.header_style.format_template("TTuple", [a, b]))
            }

            CType::Array(type_id, size) => Type::array(&self.bn_type(type_id), size as u64),

            CType::UEEnum(path) => struct_(&obj_name(self.ref_data, path)), // TODO type enum
            CType::UEStruct(path) => struct_(&obj_name(self.ref_data, path)),
            CType::UEClass(path) => struct_(&obj_name(self.ref_data, path)), // TODO type class
        }
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
                let struct_ = &self.ref_data.objects[path].get_struct().unwrap();
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
                let class = &self.ref_data.objects[class].get_class().unwrap();
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
                let enum_ = &self.ref_data.objects[path].get_enum().unwrap();
                let min = enum_.names.iter().map(|(_, v)| v).min().unwrap();
                let max = enum_.names.iter().map(|(_, v)| v).max().unwrap();
                if *min < i8::MIN as i64 || *max > u8::MAX as i64 {
                    (4, 4)
                } else {
                    (1, 1)
                }
            }
            CType::UEClass(path) | CType::UEStruct(path) => {
                let struct_ = &self.ref_data.objects[path].get_struct().unwrap();
                (struct_.properties_size, struct_.min_alignment)
            }
        }
    }

    fn decl_ctype(&mut self, buffer: &mut String, id: TypeId) {
        let ctype = self.store[id];
        let this = self.type_to_string(id, false);
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
                let data_name = self.type_to_string(data, true);
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
                let s = &mut self.store;

                let ptr_id = CType::Ptr(type_id).i(s);
                let int = CType::Int32.i(s);

                let inner = self.bn_type(ptr_id);
                let int = self.bn_type(int);

                let struct_ = Structure::builder()
                    .m(&inner, "Data", 0)
                    .m(&int, "Num", 8)
                    .m(&int, "Max", 12)
                    .finalize();

                self.bv.define_user_type(this, &Type::structure(&struct_));
            }
            CType::TMap(k, v) => {
                // struct TSet<TTuple<int,FGeneratedMissionGroup>,TDefaultMapHashableKeyFuncs<int,FGeneratedMissionGroup,0>,FDefaultSetAllocator>  {
                // /* offset 0x000 */ Elements: TSparseArray<TSetElement<TTuple<int,FGeneratedMissionGroup> >,TSparseArrayAllocator<TSizedDefaultAllocator<32>,FDefaultBitArrayAllocator> >,
                // /* offset 0x038 */ Hash: TInlineAllocator<1,TSizedDefaultAllocator<32> >::ForElementType<FSetElementId>,
                // /* offset 0x048 */ HashSize: i32,

                // struct TSparseArray<TSetElement<TTuple<int,FGeneratedMissionGroup> >,TSparseArrayAllocator<TSizedDefaultAllocator<32>,FDefaultBitArrayAllocator> >  {
                // /* offset 0x000 */ Data: TArray<TSparseArrayElementOrFreeListLink<TAlignedBytes<32,8> >,TSizedDefaultAllocator<32> >,
                // /* offset 0x010 */ AllocationFlags: TBitArray<FDefaultBitArrayAllocator>,
                // /* offset 0x030 */ FirstFreeIndex: i32,
                // /* offset 0x034 */ NumFreeIndices: i32,

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
                let a = self.type_to_string(a, true);
                let b = self.type_to_string(b, true);

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
                let enum_ = &self.ref_data.objects[path].get_enum().unwrap();
                let min = enum_.names.iter().map(|(_, v)| v).min().unwrap();
                let max = enum_.names.iter().map(|(_, v)| v).max().unwrap();
                let type_ = if *min < i8::MIN as i64 || *max > u8::MAX as i64 {
                    "uint32_t"
                } else {
                    "uint8_t"
                };
                let enum_name = obj_name(self.ref_data, path);
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
                let struct_ = &self.ref_data.objects[path].get_struct().unwrap();
                let name = obj_name(self.ref_data, path);

                // TODO class or struct

                let mut builder = Structure::builder();

                if let Some(parent) = &struct_.super_struct {
                    let parent_struct = &self.ref_data.objects[parent].get_struct().unwrap();
                    let parent = NamedTypeReference::new(
                        NamedTypeReferenceClass::StructNamedTypeClass,
                        obj_name(self.ref_data, parent),
                    );
                    builder.base_structures(&[BaseStructure {
                        ty: parent,
                        offset: 0,
                        width: parent_struct.properties_size as u64,
                    }]);
                }

                if let Some(_class) = self.ref_data.objects.get(path).unwrap().get_class() {
                    let vtable_name = format!("{name}::VTable");
                    let vtable = Type::named_type(&NamedTypeReference::new(
                        NamedTypeReferenceClass::StructNamedTypeClass,
                        vtable_name,
                    ));
                    let vtable_ptr =
                        Type::pointer(&CoreArchitecture::by_name("x86_64").unwrap(), &vtable);

                    builder.insert(
                        &vtable_ptr,
                        "vtable",
                        0,
                        false,
                        MemberAccess::PublicAccess,
                        MemberScope::NoScope, // virtual scope? or does that apply only to functions
                    );
                }

                self.decl_props(&mut builder, struct_);

                self.bv
                    .define_user_type(name, &Type::structure(&builder.finalize()));
            }
        }
    }

    fn decl_props(&mut self, bn_struct: &mut StructureBuilder, struct_: &'ref_data Struct) {
        for prop in &struct_.properties {
            let ctype = self.prop_ctype(prop);

            bn_struct.insert(
                &self.bn_type(ctype),
                prop.name.clone(),
                prop.offset as u64,
                false,
                MemberAccess::PublicAccess,
                MemberScope::NoScope,
            );
        }
    }

    fn generate(&mut self, filter: impl Fn(&str, &ObjectType) -> bool) {
        let mut buffer = String::new();

        match self.header_style {
            HeaderStyle::Binja => {}
            HeaderStyle::C => {
                writeln!(&mut buffer, "#include <stdint.h>\n").unwrap();
            }
        }

        // create vtable structs
        // add vtable member
        //
        // find common vtable members to infer owner

        let image_base = self.bv.original_image_base();
        let og_base = self.ref_data.image_base_address;

        fn get_class<'a>(ref_data: &'a ReflectionData, class: &str) -> &'a Class {
            ref_data.objects.get(class).unwrap().get_class().unwrap()
        }
        fn get_parent_in<'a>(
            ref_data: &'a ReflectionData,
            mut class: &'a str,
            in_set: &HashSet<&'a str>,
        ) -> &'a str {
            loop {
                let class_obj = get_class(ref_data, class);
                if let Some(parent) = class_obj.r#struct.super_struct.as_deref() {
                    if in_set.contains(parent) {
                        class = parent;
                        continue;
                    }
                }
                break;
            }
            class
        }

        let mut vtable_func_map: HashMap<u64, HashMap<usize, HashSet<&str>>> = Default::default();

        {
            fn vtable_len(ref_data: &ReflectionData, class: &str) -> usize {
                let mut class = Some(class);
                while let Some(next) = class {
                    let obj = ref_data.objects.get(next).unwrap().get_class().unwrap();
                    if let Some(vtable) = obj.instance_vtable {
                        return ref_data.vtables.get(&vtable).unwrap().len();
                    }
                    class = obj.r#struct.super_struct.as_deref();
                }
                0
            }

            for (path, obj) in &self.ref_data.objects {
                let Some(class) = obj.get_class() else {
                    continue;
                };

                let name = obj_name(self.ref_data, path);

                {
                    let mut builder = Structure::builder();
                    builder.propagates_data_var_refs(true);

                    let len = vtable_len(self.ref_data, path);
                    let parent_len = if let Some(parent) = &class.r#struct.super_struct {
                        let parent_name = obj_name(self.ref_data, parent);
                        let parent_type = NamedTypeReference::new(
                            NamedTypeReferenceClass::StructNamedTypeClass,
                            format!("{parent_name}::VTable"),
                        );
                        let parent_len = vtable_len(self.ref_data, parent);
                        builder.base_structures(&[BaseStructure {
                            ty: parent_type,
                            offset: 0,
                            width: 8 * parent_len as u64,
                        }]);
                        parent_len
                    } else {
                        0
                    };

                    builder.width(8 * len as u64);

                    //let vtable_name = format!("{name}::VTable");
                    //let vtable = Type::named_type(&NamedTypeReference::new(
                    //    NamedTypeReferenceClass::StructNamedTypeClass,
                    //    vtable_name,
                    //));

                    for i in parent_len..len {
                        let offset = i as u64 * 8;
                        let func = Type::function(&Type::void(), vec![], false);
                        let func_ptr =
                            Type::pointer(&CoreArchitecture::by_name("x86_64").unwrap(), &func);
                        //let func_name = format!("{outer_name}::exec{func_name}");
                        //let sym = Symbol::builder(SymbolType::Function, &func_name, addr).create();
                        //self.bv.define_user_symbol(&sym);
                        builder.insert(
                            &func_ptr,
                            format!("vfunc_0x{offset:x}"),
                            offset,
                            false,
                            MemberAccess::PublicAccess,
                            MemberScope::NoScope, // virtual scope?
                        );
                    }

                    self.bv.define_user_type(
                        format!("{name}::VTable"),
                        &Type::structure(&builder.finalize()),
                    );
                }

                if let Some(vtable) = class.instance_vtable {
                    let vtable_addr = vtable - og_base + image_base;
                    let sym =
                        Symbol::builder(SymbolType::Data, &format!("{name}::vtable"), vtable_addr)
                            .create();
                    self.bv.define_user_symbol(&sym);

                    let vtable_type = NamedTypeReference::new(
                        NamedTypeReferenceClass::StructNamedTypeClass,
                        format!("{name}::VTable"),
                    );

                    self.bv
                        .define_user_data_var(vtable_addr, &Type::named_type(&vtable_type));

                    for (i, func) in self
                        .ref_data
                        .vtables
                        .get(&vtable)
                        .unwrap()
                        .iter()
                        .enumerate()
                    {
                        vtable_func_map
                            .entry(*func)
                            .or_default()
                            .entry(i)
                            .or_default()
                            .insert(path);
                    }
                }
            }

            // define symbols for functions belonging to a single parent class
            for (func, refs) in vtable_func_map {
                if refs.len() != 1 {
                    continue;
                }
                let (index, refs) = refs.iter().next().unwrap();
                let mut roots = HashSet::new();
                for r in refs {
                    roots.insert(get_parent_in(self.ref_data, r, refs));
                }
                if roots.len() != 1 {
                    continue;
                }
                let owner = roots.iter().next().unwrap();

                let owner_name = obj_name(self.ref_data, owner);
                let func_name = format!("{owner_name}::vfunc_0x{:x}", 8 * index);
                let func_addr = func - og_base + image_base;
                let sym = Symbol::builder(SymbolType::Function, &func_name, func_addr).create();
                self.bv.define_user_symbol(&sym);
            }
        }

        let mut to_visit = HashSet::new();
        let mut dep_graph = HashMap::new();
        // get dependencies of initial top level classes
        for (path, obj) in &self.ref_data.objects {
            match obj {
                ObjectType::Object(object) => {}
                ObjectType::Package(package) => {}
                ObjectType::Enum(_) => {}
                ObjectType::ScriptStruct(script_struct) => {}
                ObjectType::Class(class) => {}
                ObjectType::Function(function) => {
                    let addr = function.func - og_base + image_base;

                    let Some(outer) = function.r#struct.object.outer.as_deref() else {
                        continue;
                    };

                    let outer_name = obj_name(self.ref_data, outer);
                    let func_name = obj_name(self.ref_data, path);
                    let func_name = format!("{outer_name}::exec{func_name}");

                    let sym = Symbol::builder(SymbolType::Function, &func_name, addr).create();

                    self.bv.define_user_symbol(&sym);
                }
            }
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

        let sorted = topological_sort(&dep_graph).unwrap();
        dbg!(&sorted);

        // full declarations
        for (dep_type, type_id) in &sorted {
            if *dep_type == DepType::Full {
                self.decl_ctype(&mut buffer, *type_id);
            }
        }

        println!("{buffer}");
    }
}

trait StructureBuilderExt {
    fn m<
        'a,
        S: binaryninja::string::BnStrCompatible,
        T: Into<binaryninja::confidence::Conf<&'a Type>>,
    >(
        &mut self,
        ty: T,
        name: S,
        offset: u64,
    ) -> &mut Self;
}

impl StructureBuilderExt for StructureBuilder {
    fn m<
        'a,
        S: binaryninja::string::BnStrCompatible,
        T: Into<binaryninja::confidence::Conf<&'a Type>>,
    >(
        &mut self,
        ty: T,
        name: S,
        offset: u64,
    ) -> &mut Self {
        self.insert(
            ty,
            name,
            offset,
            false,
            MemberAccess::PublicAccess,
            MemberScope::NoScope,
        )
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct TypeId(NonZero<usize>);
impl std::fmt::Debug for TypeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "TypeId({})", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum CType<'a, T = TypeId> {
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

    Bool(T, usize), // TODO bitfield

    FName,
    FString,
    FText,
    FFieldPath,
    MulticastInlineDelegate,
    MulticastSparseDelegate,
    Delegate,

    TArray(T),
    TMap(T, T),
    TSet(T),
    Ptr(T),
    TWeakObjectPtr(T),
    TSoftObjectPtr(T),
    TLazyObjectPtr(T),
    TScriptInterface(T),

    TTuple(T, T),

    Array(T, usize),

    UEEnum(&'a str),
    UEStruct(&'a str),
    UEClass(&'a str),
}
impl<'a> CType<'a> {
    fn i(&self, store: &mut TypeStore<'a>) -> TypeId {
        store.insert(*self)
    }
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
    fn escaped_name(&self, escape: bool) -> String {
        if self.primitive || self.pointer || !escape {
            self.name.to_string()
        } else {
            format!("`{}`", self.name)
        }
    }
}

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

fn type_fstring_data(s: &mut TypeStore<'_>) -> TypeId {
    CType::TArray(CType::WChar.i(s)).i(s)
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
}
