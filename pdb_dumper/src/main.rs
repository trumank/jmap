use anyhow::Result;
use pdb::{
    FallibleIterator as _, SymbolData, TypeData, TypeFinder, TypeIndex, TypeInformation, PDB,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fs::File;
use std::path::Path;

#[derive(Serialize, Deserialize)]
struct StructMember {
    name: String,
    offset: u64,
    size: u64,
    //type_name: String,
}

#[derive(Serialize, Deserialize)]
struct StructInfo {
    name: String,
    size: u64,
    members: Vec<StructMember>,
}

fn main() -> Result<()> {
    let whitelist = HashSet::from([
        "FName",
        "UObject",
        "UObjectBase",
        "UProperty",
        "UClass",
        "UStruct",
        "UScriptStruct",
        "UFunction",
        "UEnum",
        "FFieldClass",
        "FField",
        "FProperty",
        "FBoolProperty",
        "FObjectProperty",
        "FSoftObjectProperty",
        "FWeakObjectProperty",
        "FLazyObjectProperty",
        "FInterfaceProperty",
        "FArrayProperty",
        "FStructProperty",
        "FMapProperty",
        "FSetProperty",
        "FEnumProperty",
        "FByteProperty",
        "FObjectPropertyBase", // for FObjectProperty
    ]);

    let pdb_path = std::env::args().nth(1).unwrap();
    let output_path = "struct_info.json";

    let file = File::open(pdb_path)?;
    let mut pdb = PDB::open(file)?;

    let type_information = pdb.type_information()?;
    let mut type_finder = type_information.finder();

    let mut structs = Vec::new();
    let mut iter = type_information.iter();
    while let Some(_) = iter.next()? {
        type_finder.update(&iter);
    }

    let mut iter = type_information.iter();
    while let Some(type_item) = iter.next()? {
        if let Ok(TypeData::Class(class)) = type_item.parse() {
            let struct_name = class.name.to_string();

            if !whitelist.contains(&*struct_name) || class.properties.forward_reference() {
                continue;
            }

            let mut members = Vec::new();

            dbg!(&class);

            for field in class.fields {
                let field = type_finder.find(field)?.parse()?;
                let member_type_name = match field.name() {
                    Some(name) => name.to_string().to_string(),
                    None => format!("Unknown_{:?}", field),
                };

                match &field {
                    TypeData::Member(member_type) => {
                        todo!()
                        //members.push(StructMember {
                        //    name: field.name().unwrap().to_string().into(),
                        //    offset: member_type.offset,
                        //    size: get_type_size(&mut type_finder, member_type.field_type)?,
                        //    type_name: member_type.name.to_string().into(),
                        //});
                    }
                    TypeData::StaticMember(static_member_type) => todo!(),
                    TypeData::FieldList(data) => {
                        for field in &data.fields {
                            match &field {
                                TypeData::Member(member_type) => {
                                    let field_type =
                                        type_finder.find(member_type.field_type)?.parse()?;
                                    //dbg!(field_type);
                                    members.push(StructMember {
                                        name: field.name().unwrap().to_string().into(),
                                        offset: member_type.offset,
                                        size: get_type_size(
                                            &mut type_finder,
                                            member_type.field_type,
                                        )?,
                                        //type_name: member_type.name.to_string().into(),
                                    });
                                }
                                TypeData::Method(_)
                                | TypeData::OverloadedMethod(_)
                                | TypeData::StaticMember(_)
                                | TypeData::BaseClass(_)
                                | TypeData::Nested(_)
                                | TypeData::VirtualFunctionTablePointer(_) => {}
                                _ => todo!("{field:?}"),
                            }
                        }
                        // TODO continuation
                    }
                    _ => todo!(),
                }
            }

            structs.push(StructInfo {
                name: struct_name.to_string(),
                size: class.size,
                members,
            });
        }
    }

    // Sort members by offset for better readability
    for struct_info in &mut structs {
        struct_info.members.sort_by_key(|m| m.offset);
    }

    // Write to JSON file
    let output_file = File::create(output_path)?;
    serde_json::to_writer(output_file, &structs)?;

    println!("Successfully wrote struct information to {}", output_path);
    Ok(())
}

fn get_type_size(type_finder: &mut TypeFinder, type_index: TypeIndex) -> Result<u64> {
    let type_item = type_finder.find(type_index)?.parse()?;
    Ok(match type_item {
        TypeData::Primitive(primitive_type) => {
            match primitive_type.kind {
                //pdb::PrimitiveKind::NoType => todo!(),
                pdb::PrimitiveKind::Void => 0,
                pdb::PrimitiveKind::Char => 1,
                pdb::PrimitiveKind::UChar => 1,
                //pdb::PrimitiveKind::RChar => todo!(),
                //pdb::PrimitiveKind::WChar => 2,
                //pdb::PrimitiveKind::RChar16 => todo!(),
                //pdb::PrimitiveKind::RChar32 => todo!(),
                pdb::PrimitiveKind::I8 => 1,
                pdb::PrimitiveKind::U8 => 1,
                pdb::PrimitiveKind::Short => 2,
                pdb::PrimitiveKind::UShort => 2,
                pdb::PrimitiveKind::I16 => 2,
                pdb::PrimitiveKind::U16 => 2,
                //pdb::PrimitiveKind::Long => todo!(),
                //pdb::PrimitiveKind::ULong => todo!(),
                pdb::PrimitiveKind::I32 => 4,
                pdb::PrimitiveKind::U32 => 4,
                pdb::PrimitiveKind::Quad => 8,
                pdb::PrimitiveKind::UQuad => 8,
                pdb::PrimitiveKind::I64 => 8,
                pdb::PrimitiveKind::U64 => 8,
                //pdb::PrimitiveKind::Octa => todo!(),
                //pdb::PrimitiveKind::UOcta => todo!(),
                pdb::PrimitiveKind::I128 => 16,
                pdb::PrimitiveKind::U128 => 16,
                pdb::PrimitiveKind::F16 => 2,
                pdb::PrimitiveKind::F32 => 4,
                pdb::PrimitiveKind::F32PP => 4,
                //pdb::PrimitiveKind::F48 => todo!(),
                pdb::PrimitiveKind::F64 => 8,
                //pdb::PrimitiveKind::F80 => todo!(),
                pdb::PrimitiveKind::F128 => 16,
                //pdb::PrimitiveKind::Complex32 => todo!(),
                //pdb::PrimitiveKind::Complex64 => todo!(),
                //pdb::PrimitiveKind::Complex80 => todo!(),
                //pdb::PrimitiveKind::Complex128 => todo!(),
                pdb::PrimitiveKind::Bool8 => 1,
                pdb::PrimitiveKind::Bool16 => 2,
                pdb::PrimitiveKind::Bool32 => 4,
                pdb::PrimitiveKind::Bool64 => 8,
                //pdb::PrimitiveKind::HRESULT => todo!(),
                _ => todo!("{:?}", primitive_type.kind),
            }
        }
        TypeData::Class(class_type) => class_type.size, // TODO handle forward references
        //TypeData::Member(member_type) => todo!(),
        //TypeData::MemberFunction(member_function_type) => todo!(),
        //TypeData::OverloadedMethod(overloaded_method_type) => todo!(),
        //TypeData::Method(method_type) => todo!(),
        //TypeData::StaticMember(static_member_type) => todo!(),
        //TypeData::Nested(nested_type) => todo!(),
        //TypeData::BaseClass(base_class_type) => todo!(),
        //TypeData::VirtualBaseClass(virtual_base_class_type) => todo!(),
        //TypeData::VirtualFunctionTablePointer(virtual_function_table_pointer_type) => todo!(),
        //TypeData::Procedure(procedure_type) => todo!(),
        TypeData::Pointer(pointer_type) => {
            match pointer_type.attributes.pointer_kind() {
                //pdb::PointerKind::Near16 => todo!(),
                //pdb::PointerKind::Far16 => todo!(),
                //pdb::PointerKind::Huge16 => todo!(),
                //pdb::PointerKind::BaseSeg => todo!(),
                //pdb::PointerKind::BaseVal => todo!(),
                //pdb::PointerKind::BaseSegVal => todo!(),
                //pdb::PointerKind::BaseAddr => todo!(),
                //pdb::PointerKind::BaseSegAddr => todo!(),
                //pdb::PointerKind::BaseType => todo!(),
                //pdb::PointerKind::BaseSelf => todo!(),
                //pdb::PointerKind::Near32 => todo!(),
                //pdb::PointerKind::Far32 => todo!(),
                pdb::PointerKind::Ptr64 => 8,
                k => todo!("{k:?}"),
            }
        }
        //TypeData::Modifier(modifier_type) => todo!(),
        TypeData::Enumeration(enumeration_type) => {
            get_type_size(type_finder, enumeration_type.underlying_type)?
        }
        //TypeData::Enumerate(enumerate_type) => todo!(),
        //TypeData::Array(array_type) => todo!(),
        //TypeData::Union(union_type) => todo!(),
        TypeData::Bitfield(bitfield_type) => {
            get_type_size(type_finder, bitfield_type.underlying_type)?
        }
        //TypeData::FieldList(field_list) => todo!(),
        //TypeData::ArgumentList(argument_list) => todo!(),
        //TypeData::MethodList(method_list) => todo!(),
        t => todo!("{t:?}"),
    })
}
