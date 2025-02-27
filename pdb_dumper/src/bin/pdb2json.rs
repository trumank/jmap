use anyhow::{bail, Result};
use dynamic_structs::{BaseClass, ClassInfo, ClassMember, OutputData, VTableEntry};
use pdb::{FallibleIterator, TypeIndex};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};
use std::fs::File;
use std::io::Write;

type TypeSet = BTreeSet<pdb::TypeIndex>;

fn type_name(
    type_finder: &pdb::TypeFinder<'_>,
    type_index: pdb::TypeIndex,
    needed_types: &mut TypeSet,
) -> Result<String> {
    let mut name = match type_finder.find(type_index)?.parse()? {
        pdb::TypeData::Primitive(data) => {
            let mut name = match data.kind {
                pdb::PrimitiveKind::Void => "void".to_string(),
                pdb::PrimitiveKind::Char => "char".to_string(),
                pdb::PrimitiveKind::UChar => "unsigned char".to_string(),

                pdb::PrimitiveKind::I8 => "int8_t".to_string(),
                pdb::PrimitiveKind::U8 => "uint8_t".to_string(),
                pdb::PrimitiveKind::I16 => "int16_t".to_string(),
                pdb::PrimitiveKind::U16 => "uint16_t".to_string(),
                pdb::PrimitiveKind::I32 => "int32_t".to_string(),
                pdb::PrimitiveKind::U32 => "uint32_t".to_string(),
                pdb::PrimitiveKind::I64 => "int64_t".to_string(),
                pdb::PrimitiveKind::U64 => "uint64_t".to_string(),

                pdb::PrimitiveKind::F32 => "float".to_string(),
                pdb::PrimitiveKind::F64 => "double".to_string(),

                pdb::PrimitiveKind::Bool8 => "bool".to_string(),

                _ => format!("unhandled_primitive.kind /* {:?} */", data.kind),
            };

            if data.indirection.is_some() {
                name.push_str(" *");
            }

            name
        }

        pdb::TypeData::Class(data) => {
            needed_types.insert(type_index);
            data.name.to_string().into_owned()
        }

        pdb::TypeData::Enumeration(data) => {
            needed_types.insert(type_index);
            data.name.to_string().into_owned()
        }

        pdb::TypeData::Union(data) => {
            needed_types.insert(type_index);
            data.name.to_string().into_owned()
        }

        pdb::TypeData::Pointer(data) => format!(
            "{}*",
            type_name(type_finder, data.underlying_type, needed_types)?
        ),

        pdb::TypeData::Modifier(data) => {
            if data.constant {
                format!(
                    "const {}",
                    type_name(type_finder, data.underlying_type, needed_types)?
                )
            } else if data.volatile {
                format!(
                    "volatile {}",
                    type_name(type_finder, data.underlying_type, needed_types)?
                )
            } else {
                // ?
                type_name(type_finder, data.underlying_type, needed_types)?
            }
        }

        pdb::TypeData::Array(data) => {
            let mut name = type_name(type_finder, data.element_type, needed_types)?;
            for size in data.dimensions {
                name = format!("{}[{}]", name, size);
            }
            name
        }

        _ => format!("Type{} /* TODO: figure out how to name it */", type_index),
    };

    // TODO: search and replace std:: patterns
    if name == "std::basic_string<char,std::char_traits<char>,std::allocator<char> >" {
        name = "std::string".to_string();
    }

    Ok(name)
}

struct ClassBuilder<'p> {
    class_info: ClassInfo,
    vtable_index: usize,
    type_finder: &'p pdb::TypeFinder<'p>,
    needed_types: &'p mut TypeSet,
    base_class_types: HashSet<pdb::TypeIndex>,
}

impl<'p> ClassBuilder<'p> {
    fn new(
        name: String,
        kind: pdb::ClassKind,
        type_finder: &'p pdb::TypeFinder<'p>,
        needed_types: &'p mut TypeSet,
    ) -> Self {
        ClassBuilder {
            class_info: ClassInfo {
                name,
                kind: match kind {
                    pdb::ClassKind::Class => "class".to_string(),
                    pdb::ClassKind::Struct => "struct".to_string(),
                    pdb::ClassKind::Interface => "interface".to_string(),
                },
                base_classes: Vec::new(),
                members: Vec::new(),
                vtable: Vec::new(),
                size: 0,
            },
            vtable_index: 0,
            type_finder,
            needed_types,
            base_class_types: Default::default(),
        }
    }

    fn add_base_class(&mut self, data: &pdb::BaseClassType) -> Result<()> {
        self.base_class_types.insert(data.base_class);
        self.class_info.base_classes.push(BaseClass {
            name: type_name(self.type_finder, data.base_class, self.needed_types)?,
            offset: data.offset,
            virtual_base: false,
        });
        Ok(())
    }

    fn add_virtual_base_class(&mut self, data: &pdb::VirtualBaseClassType) -> Result<()> {
        self.base_class_types.insert(data.base_class);
        self.class_info.base_classes.push(BaseClass {
            name: type_name(self.type_finder, data.base_class, self.needed_types)?,
            offset: data.base_pointer_offset,
            virtual_base: true,
        });
        Ok(())
    }

    fn add_member(&mut self, data: &pdb::MemberType) -> Result<()> {
        self.class_info.members.push(ClassMember {
            name: data.name.to_string().into_owned(),
            type_name: type_name(self.type_finder, data.field_type, self.needed_types)?,
            offset: data.offset,
        });
        Ok(())
    }

    fn add_method(&mut self, data: &pdb::MethodType, attrs: pdb::FieldAttributes) -> Result<()> {
        if attrs.is_virtual() {
            if let Ok(method_data) = self.type_finder.find(data.method_type)?.parse() {
                if let pdb::TypeData::MemberFunction(func_data) = method_data {
                    let entry = VTableEntry {
                        index: self.vtable_index,
                        name: data.name.to_string().into_owned(),
                        return_type: type_name(
                            self.type_finder,
                            func_data.return_type,
                            self.needed_types,
                        )?,
                        arguments: argument_list(
                            self.type_finder,
                            func_data.argument_list,
                            self.needed_types,
                        )?,
                    };
                    self.class_info.vtable.push(entry);
                    self.vtable_index += 1;
                }
            }
        }
        Ok(())
    }

    fn get_base_class_types(&self) -> HashSet<pdb::TypeIndex> {
        self.base_class_types.clone()
    }

    fn build(self) -> (ClassInfo, HashSet<pdb::TypeIndex>) {
        (self.class_info, self.base_class_types)
    }
}

fn argument_list(
    type_finder: &pdb::TypeFinder<'_>,
    type_index: pdb::TypeIndex,
    needed_types: &mut TypeSet,
) -> Result<Vec<String>> {
    match type_finder.find(type_index)?.parse()? {
        pdb::TypeData::ArgumentList(data) => {
            let mut args: Vec<String> = Vec::new();
            for arg_type in data.arguments {
                args.push(type_name(type_finder, arg_type, needed_types)?);
            }
            Ok(args)
        }
        _ => bail!("argument list of non-argument-list type"),
    }
}

struct PdbProcessor<'p> {
    type_finder: pdb::TypeFinder<'p>,
    needed_types: TypeSet,
    processed_classes: HashSet<String>,
    class_queue: VecDeque<pdb::TypeIndex>,
    output: OutputData,
}

impl<'p> PdbProcessor<'p> {
    fn new(type_finder: pdb::TypeFinder<'p>) -> Self {
        PdbProcessor {
            type_finder,
            needed_types: TypeSet::new(),
            processed_classes: HashSet::new(),
            class_queue: Default::default(),
            output: OutputData {
                classes: Vec::new(),
            },
        }
    }

    fn process_fields(
        type_finder: &pdb::TypeFinder<'_>,
        type_index: pdb::TypeIndex,
        builder: &mut ClassBuilder<'_>,
    ) -> Result<()> {
        match type_finder.find(type_index)?.parse()? {
            pdb::TypeData::FieldList(data) => {
                for field in &data.fields {
                    match field {
                        pdb::TypeData::Member(data) => {
                            builder.add_member(data)?;
                        }
                        pdb::TypeData::Method(data) => {
                            builder.add_method(data, data.attributes)?;
                        }
                        pdb::TypeData::BaseClass(data) => {
                            builder.add_base_class(data)?;
                        }
                        pdb::TypeData::VirtualBaseClass(data) => {
                            builder.add_virtual_base_class(data)?;
                        }
                        _ => {}
                    }
                }

                if let Some(continuation) = data.continuation {
                    Self::process_fields(type_finder, continuation, builder)?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn queue_class(&mut self, type_index: pdb::TypeIndex) {
        if !self.class_queue.contains(&type_index) {
            self.class_queue.push_back(type_index);
        }
    }

    fn process_class(&mut self, type_index: pdb::TypeIndex) -> Result<()> {
        if let Ok(type_data) = self.type_finder.find(type_index)?.parse() {
            if let pdb::TypeData::Class(class) = type_data {
                let class_name = class.name.to_string().into_owned();

                // Skip if already processed or is a forward reference
                if self.processed_classes.contains(&class_name)
                    || class.properties.forward_reference()
                {
                    return Ok(());
                }

                let mut builder = ClassBuilder::new(
                    class_name.clone(),
                    class.kind,
                    &self.type_finder,
                    &mut self.needed_types,
                );

                // Process class fields and collect base classes
                if let Some(fields) = class.fields {
                    Self::process_fields(&self.type_finder, fields, &mut builder)?;

                    // Queue any base classes found
                    for base_type in builder.get_base_class_types() {
                        if !self.class_queue.contains(&base_type) {
                            self.class_queue.push_back(base_type);
                        }
                    }
                }

                self.processed_classes.insert(class_name);
                let (class_info, _) = builder.build();
                self.output.classes.push(class_info);
            }
        }
        Ok(())
    }

    fn process_queue(&mut self, forward_map: &HashMap<TypeIndex, TypeIndex>) -> Result<()> {
        while let Some(type_index) = self.class_queue.pop_front() {
            self.process_class(*forward_map.get(&type_index).unwrap())?;
        }
        Ok(())
    }
}

fn write_classes(filename: &str, class_names: &[&str], output_file: &str) -> Result<()> {
    let file = std::fs::File::open(filename)?;
    let mut pdb = pdb::PDB::open(file)?;

    let type_information = pdb.type_information()?;
    let type_finder = type_information.finder();
    let mut processor = PdbProcessor::new(type_finder);

    let mut name_map = HashMap::<_, Vec<_>>::new();
    let mut forward_map = HashMap::new();
    let mut type_iter = type_information.iter();
    while let Some(typ) = type_iter.next()? {
        processor.type_finder.update(&type_iter);
        if let Ok(pdb::TypeData::Class(class)) = typ.parse() {
            let entries = name_map.entry(class.name).or_default();
            entries.push(typ.index());
            if !class.properties.forward_reference() {
                for t in entries.into_iter() {
                    forward_map.insert(t.clone(), typ.index());
                }
                if class_names.contains(&&*class.name.to_string()) {
                    processor.queue_class(typ.index());
                }
            }
        }
    }

    processor.process_queue(&forward_map)?;

    // Write output
    let mut file = File::create(output_file)?;
    serde_json::to_writer_pretty(&mut file, &processor.output)?;
    Ok(())
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 4 {
        println!(
            "Usage: {} <input.pdb> <output.json> <class1> [class2 ...]",
            args[0]
        );
        return;
    }

    let filename = &args[1];
    let output_file = &args[2];
    let class_names = args[3..].to_vec();

    match write_classes(
        filename,
        class_names
            .iter()
            .map(String::as_ref)
            .collect::<Vec<_>>()
            .as_slice(),
        output_file,
    ) {
        Ok(_) => println!("Successfully wrote class data to {}", output_file),
        Err(e) => eprintln!("Error processing PDB: {}", e),
    }
}
