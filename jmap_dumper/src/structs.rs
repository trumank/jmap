use std::collections::HashMap;

use anyhow::{Context, Result, anyhow, bail};
use gospel_compiler::backend::{CompilerInstance, CompilerModuleBuilder, CompilerResultTrait};
use gospel_compiler::parser::parse_source_file;
use gospel_typelib::type_model::{
    ResolvedUDTMemberLayout, TargetTriplet, Type, TypeGraphLike, TypeLayoutCache,
};
use gospel_vm::vm::{GospelVMOptions, GospelVMRunContext, GospelVMState, GospelVMValue};
use patternsleuth::resolvers::unreal::engine_version::EngineVersion;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Structs(pub Vec<StructInfo>);

#[derive(Serialize, Deserialize, Clone)]
pub struct StructInfo {
    pub name: String,
    pub size: u64,
    pub alignment: u64,
    pub members: Vec<StructMember>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct StructMember {
    pub name: String,
    pub offset: u64,
    pub size: u64,
    //pub type_name: String,
}

pub fn get_struct_info_for_version(
    version: &EngineVersion,
    case_preserving: bool,
) -> Result<Structs> {
    let target_triplet = TargetTriplet {
        arch: gospel_typelib::type_model::TargetArchitecture::X86_64,
        sys: gospel_typelib::type_model::TargetOperatingSystem::Win32,
        env: gospel_typelib::type_model::TargetEnvironment::MSVC,
    };

    let compiler_instance = CompilerInstance::create(Default::default());
    let module_writer = compiler_instance
        .define_module("unreal")
        .to_simple_result()?;

    let module_source_file = parse_source_file("unreal.gs", include_str!("unreal.gs"))?;

    module_writer
        .add_source_file(module_source_file)
        .to_simple_result()?;

    let mut vm_state = GospelVMState::create();

    let ue_version = (version.major as u64) * 100 + (version.minor as u64);
    let case_preserving_flag = if case_preserving { 1 } else { 0 };

    let struct_names = vec![
        "FUObjectArray",
        "FUObjectArrayOld",
        "FUObjectArrayOlder",
        "FUObjectItem",
        "FFixedUObjectArray",
        "FChunkedFixedUObjectArray",
        "UObject",
        "UField",
        "UStruct",
        "UClass",
        "UEnum",
        "UEnumNameTuple",
        "UFunction",
        "UScriptStruct",
        "ZField",
        "ZProperty",
        "ZStructProperty",
        "ZArrayProperty",
        "ZEnumProperty",
        "ZByteProperty",
        "ZBoolProperty",
        "ZSetProperty",
        "ZMapProperty",
        "ZDelegateProperty",
        "ZMulticastDelegateProperty",
        "ZObjectPropertyBase",
        "ZObjectProperty",
        "ZClassProperty",
        "ZSoftClassProperty",
        "ZInterfaceProperty",
        "FOptionalPropertyLayout",
        "FName",
        "FField",
        "FFieldClass",
    ];

    let container = module_writer.compile().to_simple_result()?;

    let mounted_container = vm_state.mount_container(container)?;

    let mut structs = Vec::new();
    let vm_options = GospelVMOptions::default()
        .target_triplet(target_triplet.clone())
        .with_global("UE_VERSION", ue_version)
        .with_global("WITH_CASE_PRESERVING_NAME", case_preserving_flag);
    let mut execution_context = GospelVMRunContext::create(vm_options);

    for struct_name in struct_names {
        if let Some(struct_info) = eval_struct_layout(
            &mounted_container,
            &mut execution_context,
            &target_triplet,
            struct_name,
        )? {
            structs.push(struct_info);
        }
    }

    Ok(Structs(structs))
}

fn eval_struct_layout(
    container: &std::rc::Rc<gospel_vm::vm::GospelVMContainer>,
    execution_context: &mut GospelVMRunContext,
    target_triplet: &TargetTriplet,
    struct_name: &str,
) -> Result<Option<StructInfo>> {
    let eval_func = container
        .find_named_function(&format!("unreal${struct_name}"))
        .with_context(|| format!("{struct_name} not found in module"))?;
    match eval_func
        .execute(Vec::new(), execution_context)
        .map_err(|e| anyhow!("GospelVM exec failed: {e}"))?
    {
        GospelVMValue::TypeReference(type_index) => {
            let type_tree = execution_context.type_tree(type_index);

            let mut members = HashMap::new();
            let mut layout_cache = TypeLayoutCache::create(target_triplet.clone());

            let mut add_members = |type_index: usize, base: usize| -> Result<Vec<(usize, usize)>> {
                let Type::UDT(udt) = &type_tree.types[type_index] else {
                    bail!("Expected UserDefinedType");
                };
                let layout = udt.layout(type_index, &type_tree, &mut layout_cache)?;

                for (member, member_layout) in udt.members.iter().zip(&layout.member_layouts) {
                    if let Some(name) = member.name()
                        && !members.contains_key(name)
                        && let ResolvedUDTMemberLayout::Field(field) = member_layout
                    {
                        members.insert(
                            name,
                            StructMember {
                                name: member.name().unwrap_or("<unknown>").to_string(),
                                offset: (base + field.offset) as u64,
                                size: field.size as u64,
                            },
                        );
                    }
                }

                Ok(udt
                    .base_class_indices
                    .iter()
                    .copied()
                    .zip(layout.base_class_offsets.iter().copied())
                    .collect())
            };

            let mut queue = vec![(type_tree.root_type_index, 0)];
            while let Some((type_index, base)) = queue.pop() {
                queue.extend(add_members(type_index, base)?);
            }

            let layout = match type_tree.root_type() {
                Type::UDT(udt) => udt,
                _ => unreachable!(),
            }
            .layout(type_tree.root_type_index, &type_tree, &mut layout_cache)?;

            let mut members = members.into_values().collect::<Vec<_>>();
            members.sort_by_key(|m| m.offset);

            Ok(Some(StructInfo {
                name: struct_name.to_string(),
                size: layout.size as u64,
                alignment: layout.alignment as u64,
                members,
            }))
        }
        _ => bail!("Unhandled GospelVMValue type for {struct_name}"),
    }
}
