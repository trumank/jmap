use std::collections::HashMap;

use anyhow::{Context, Result, anyhow, bail};
use gospel_compiler::backend::{CompilerInstance, CompilerModuleBuilder, CompilerResultTrait};
use gospel_compiler::parser::parse_source_file;
use gospel_typelib::target_triplet::{
    TargetArchitecture, TargetEnvironment, TargetOperatingSystem, TargetTriplet,
};
use gospel_typelib::type_model::{ResolvedUDTMemberLayout, Type, TypeGraphLike, TypeLayoutCache};
use gospel_vm::vm::{GospelVMOptions, GospelVMRunContext, GospelVMState, GospelVMValue};
use include_dir::{Dir, include_dir};
use patternsleuth::resolvers::unreal::engine_version::EngineVersion;
use serde::{Deserialize, Serialize};

static UNREAL_MODULE_SRC: Dir = include_dir!("$CARGO_MANIFEST_DIR/unreal/src");

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
        arch: TargetArchitecture::X86_64,
        sys: TargetOperatingSystem::Win32,
        env: Some(TargetEnvironment::MSVC),
    };

    let compiler_instance = CompilerInstance::create(Default::default());
    let module_writer = compiler_instance
        .define_module("unreal")
        .to_simple_result()?;

    let mut source_files: Vec<_> = UNREAL_MODULE_SRC
        .files()
        .filter(|f| f.path().extension().is_some_and(|ext| ext == "gs"))
        .collect();
    source_files.sort_by_key(|f| f.path());

    for file in source_files {
        let filename = file.path().file_name().unwrap().to_string_lossy();
        let contents = file.contents_utf8().unwrap();
        let parsed = parse_source_file(&filename, contents)?;
        module_writer.add_source_file(parsed).to_simple_result()?;
    }

    let container = module_writer.compile().to_simple_result()?;

    let mut vm_state = GospelVMState::create();

    let ue_version = (version.major as u64) * 100 + (version.minor as u64);

    let struct_names: Vec<(&str, &str)> = vec![
        ("uobjectarray", "FUObjectArray"),
        ("uobjectarray", "FUObjectArrayOld"),
        ("uobjectarray", "FUObjectArrayOlder"),
        ("uobjectarray", "FUObjectItem"),
        ("uobjectarray", "FFixedUObjectArray"),
        ("uobjectarray", "FChunkedFixedUObjectArray"),
        ("objects", "UObject"),
        ("objects", "UField"),
        ("objects", "UStruct"),
        ("objects", "UClass"),
        ("objects", "UEnum"),
        ("objects", "UEnumNameTuple"),
        ("objects", "UFunction"),
        ("objects", "UScriptStruct"),
        ("properties", "ZField"),
        ("properties", "ZProperty"),
        ("properties", "ZStructProperty"),
        ("properties", "ZArrayProperty"),
        ("properties", "ZEnumProperty"),
        ("properties", "ZByteProperty"),
        ("properties", "ZBoolProperty"),
        ("properties", "ZSetProperty"),
        ("properties", "ZMapProperty"),
        ("properties", "ZDelegateProperty"),
        ("properties", "ZMulticastDelegateProperty"),
        ("properties", "ZObjectPropertyBase"),
        ("properties", "ZObjectProperty"),
        ("properties", "ZClassProperty"),
        ("properties", "ZSoftClassProperty"),
        ("properties", "ZInterfaceProperty"),
        ("properties", "FOptionalPropertyLayout"),
        ("properties", "FScriptSparseArrayLayout"),
        ("properties", "FScriptSetLayout"),
        ("properties", "FScriptMapLayout"),
        ("containers", "FDefaultBitArrayAllocator"),
        ("containers", "FScriptArray"),
        ("containers", "FScriptBitArray"),
        ("containers", "FScriptSparseArray"),
        ("containers", "FScriptSet"),
        ("containers", "FScriptMap"),
        ("unreal", "FName"),
        ("properties", "FField"),
        ("properties", "FFieldClass"),
    ];

    let mounted_container = vm_state.mount_container(container)?;

    let mut structs = Vec::new();
    let vm_options = GospelVMOptions::default()
        .target_triplet(target_triplet)
        .with_global("UE_VERSION", ue_version)
        .with_global("WITH_CASE_PRESERVING_NAME", case_preserving as u64);
    let mut execution_context = GospelVMRunContext::create(vm_options);

    for (file_name, struct_name) in struct_names {
        if let Some(struct_info) = eval_struct_layout(
            &mounted_container,
            &mut execution_context,
            &target_triplet,
            file_name,
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
    file_name: &str,
    struct_name: &str,
) -> Result<Option<StructInfo>> {
    let func_name = format!("{}${}", file_name, struct_name);
    let eval_func = container
        .find_named_function(&func_name)
        .with_context(|| format!("{struct_name} not found in module (looked for {func_name})"))?;
    match eval_func
        .execute(Vec::new(), execution_context)
        .map_err(|e| anyhow!("GospelVM exec failed: {e}"))?
    {
        GospelVMValue::TypeReference(type_index) => {
            let type_tree = execution_context.type_tree(type_index);

            let mut members = HashMap::new();
            let mut layout_cache = TypeLayoutCache::create(*target_triplet);

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
