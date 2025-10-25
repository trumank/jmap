use std::collections::BTreeMap;
use std::fmt::Write;

use ue_reflection::{
    EClassCastFlags, EFunctionFlags, EPropertyFlags, Function, ObjectType, Property, PropertyType,
    Jmap, Struct,
};

type Objects = BTreeMap<String, ObjectType>;

fn get_class_name(objects: &Objects, path: &str) -> String {
    let obj = &objects[path];
    let name = path.rsplit(['/', '.', ':']).next().unwrap();
    match obj {
        ObjectType::Enum(_) => name.into(),
        ObjectType::ScriptStruct(_) => format!("F{name}"),
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
        _ => name.into(),
    }
}

fn get_class_functions<'a>(
    objects: &'a Objects,
    class_obj: &'a Struct,
) -> Vec<(&'a String, &'a Function)> {
    class_obj
        .object
        .children
        .iter()
        .filter_map(|child_path| {
            if let Some(ObjectType::Function(func)) = objects.get(child_path) {
                Some((child_path, func))
            } else {
                None
            }
        })
        .collect()
}

fn property_type_name(objects: &Objects, prop: &Property) -> String {
    match &prop.r#type {
        PropertyType::Struct { r#struct } => get_class_name(objects, r#struct),
        PropertyType::Str => "FString".into(),
        PropertyType::Name => "FName".into(),
        PropertyType::Text => "FText".into(),
        PropertyType::FieldPath => "FFieldPath".into(),
        PropertyType::MulticastInlineDelegate { .. } => "FMulticastInlineDelegate".into(),
        PropertyType::MulticastSparseDelegate { .. } => "FMulticastSparseDelegate".into(),
        PropertyType::MulticastDelegate { .. } => "FMulticastDelegate".into(),
        PropertyType::Delegate { .. } => "FDelegate".into(),
        PropertyType::Bool { .. } => "bool".into(),
        PropertyType::Array { inner } => format!("TArray<{}>", property_type_name(objects, inner)),
        PropertyType::Enum { r#enum, .. } => r#enum
            .as_ref()
            .map(|e| get_class_name(objects, e))
            .unwrap_or_else(|| "uint8_t".into()),
        PropertyType::Map {
            key_prop,
            value_prop,
        } => {
            format!(
                "TMap<{}, {}>",
                property_type_name(objects, key_prop),
                property_type_name(objects, value_prop)
            )
        }
        PropertyType::Set { key_prop } => {
            format!("TSet<{}>", property_type_name(objects, key_prop))
        }
        PropertyType::Float => "float".into(),
        PropertyType::Double => "double".into(),
        PropertyType::Byte { .. } => "uint8_t".into(),
        PropertyType::UInt16 => "uint16_t".into(),
        PropertyType::UInt32 => "uint32_t".into(),
        PropertyType::UInt64 => "uint64_t".into(),
        PropertyType::Int8 => "int8_t".into(),
        PropertyType::Int16 => "int16_t".into(),
        PropertyType::Int => "int32_t".into(),
        PropertyType::Int64 => "int64_t".into(),
        PropertyType::Object { property_class } => {
            format!("{}*", get_class_name(objects, property_class))
        }
        PropertyType::Class { property_class, .. } => {
            format!("{}*", get_class_name(objects, property_class))
        }
        PropertyType::WeakObject { property_class } => format!(
            "TWeakObjectPtr<{}>",
            get_class_name(objects, property_class)
        ),
        PropertyType::SoftObject { property_class } => format!(
            "TSoftObjectPtr<{}>",
            get_class_name(objects, property_class)
        ),
        PropertyType::SoftClass { meta_class, .. } => {
            format!("TSoftClassPtr<{}>", get_class_name(objects, meta_class))
        }
        PropertyType::LazyObject { property_class } => format!(
            "TLazyObjectPtr<{}>",
            get_class_name(objects, property_class)
        ),
        PropertyType::Interface { interface_class } => format!(
            "TScriptInterface<{}>",
            get_class_name(objects, interface_class)
        ),
        PropertyType::Optional { inner } => {
            format!("TOptional<{}>", property_type_name(objects, inner))
        }
        PropertyType::Utf8Str => "char*".into(),
        PropertyType::AnsiStr => "char*".into(),
    }
}

fn generate_function(buffer: &mut String, objects: &Objects, path: &str, func: &Function) {
    let name = path.rsplit(['/', '.', ':']).next().unwrap();

    // Find return type (property with CPF_ReturnParm)
    let return_prop = func
        .r#struct
        .properties
        .iter()
        .find(|p| p.flags.contains(EPropertyFlags::CPF_ReturnParm));

    let return_type = if let Some(ret) = return_prop {
        property_type_name(objects, ret)
    } else {
        "void".to_string()
    };

    // Get parameters (CPF_Parm but not CPF_ReturnParm)
    let params: Vec<String> = func
        .r#struct
        .properties
        .iter()
        .filter(|p| {
            p.flags.contains(EPropertyFlags::CPF_Parm)
                && !p.flags.contains(EPropertyFlags::CPF_ReturnParm)
        })
        .map(|p| {
            let type_name = property_type_name(objects, p);
            let is_out = p.flags.contains(EPropertyFlags::CPF_OutParm);
            let is_const = p.flags.contains(EPropertyFlags::CPF_ConstParm);

            // Build parameter with modifiers
            let mut param = String::new();
            if is_const {
                param.push_str("const ");
            }
            param.push_str(&type_name);
            if is_out {
                param.push_str("&");
            }
            param.push(' ');
            param.push_str(&p.name);
            param
        })
        .collect();

    // Function qualifiers
    let is_static = func.function_flags.contains(EFunctionFlags::FUNC_Static);
    let is_const = func.function_flags.contains(EFunctionFlags::FUNC_Const);

    // Write function declaration
    write!(buffer, "    ").unwrap();
    if is_static {
        write!(buffer, "static ").unwrap();
    }
    write!(buffer, "{} {}(", return_type, name).unwrap();
    write!(buffer, "{}", params.join(", ")).unwrap();
    write!(buffer, ")").unwrap();

    // Add const after parameters if it's an instance method
    if is_const && !is_static {
        write!(buffer, " const").unwrap();
    }

    writeln!(buffer, ";").unwrap();
}

fn generate_struct_or_class(
    buffer: &mut String,
    objects: &Objects,
    path: &str,
    struct_obj: &Struct,
    keyword: &str,
) {
    let name = get_class_name(objects, path);

    writeln!(buffer, "// Size: 0x{:x}", struct_obj.properties_size).unwrap();
    write!(buffer, "{} {}", keyword, name).unwrap();

    if let Some(super_path) = &struct_obj.super_struct {
        let super_name = get_class_name(objects, super_path);
        write!(buffer, " : public {}", super_name).unwrap();
    }

    writeln!(buffer, " {{").unwrap();

    for prop in &struct_obj.properties {
        let type_name = property_type_name(objects, prop);
        let array_suffix = if prop.array_dim > 1 {
            format!("[{}]", prop.array_dim)
        } else {
            String::new()
        };

        writeln!(
            buffer,
            "    /* 0x{:04x} */ {} {}{};",
            prop.offset, type_name, prop.name, array_suffix
        )
        .unwrap();
    }

    let functions = get_class_functions(objects, struct_obj);
    if !functions.is_empty() {
        writeln!(buffer).unwrap();
        for (func_path, func) in functions {
            generate_function(buffer, objects, func_path, func);
        }
    }

    writeln!(buffer, "}};").unwrap();
    writeln!(buffer).unwrap();
}

pub fn into_header(reflection_data: &Jmap) -> String {
    let mut buffer = String::new();

    let objects = &reflection_data.objects;

    for (path, obj) in objects {
        match obj {
            ObjectType::Enum(enum_obj) => {
                let name = get_class_name(objects, path);
                writeln!(&mut buffer, "enum class {} {{", name).unwrap();
                for (enum_name, value) in &enum_obj.names {
                    let short_name = enum_name.rsplit("::").next().unwrap_or(enum_name);
                    writeln!(&mut buffer, "    {} = {},", short_name, value).unwrap();
                }
                writeln!(&mut buffer, "}};").unwrap();
                writeln!(&mut buffer).unwrap();
            }
            ObjectType::ScriptStruct(script_struct) => {
                generate_struct_or_class(
                    &mut buffer,
                    objects,
                    path,
                    &script_struct.r#struct,
                    "struct",
                );
            }
            ObjectType::Class(class) => {
                generate_struct_or_class(&mut buffer, objects, path, &class.r#struct, "class");
            }
            _ => {}
        }
    }

    buffer
}
