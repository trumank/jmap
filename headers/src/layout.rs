use crate::parser::{AccessSection, DataType, Declaration, Member};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct MemoryLayout {
    pub size: usize,
    pub alignment: usize,
    pub members: Vec<(String, usize)>, // (member_name, offset)
}

#[derive(Debug, Clone)]
pub enum LayoutType {
    Int,
    Float,
    Double,
    Char,
    Bool,
    Void,
    Custom(String),
    Pointer(Box<LayoutType>),
    Reference(Box<LayoutType>),
    Template(String, Vec<LayoutType>),
}

#[derive(Debug, Clone)]
pub struct LayoutMember {
    pub name: String,
    pub data_type: LayoutType,
}

#[derive(Debug, Clone)]
pub struct LayoutSection {
    pub members: Vec<LayoutMember>,
}

#[derive(Debug, Clone)]
pub struct LayoutDeclaration {
    pub name: String,
    pub sections: Vec<LayoutSection>,
}

impl LayoutType {
    fn size_and_alignment(&self) -> (usize, usize) {
        match self {
            LayoutType::Int => (4, 4),
            LayoutType::Float => (4, 4),
            LayoutType::Double => (8, 8),
            LayoutType::Char => (1, 1),
            LayoutType::Bool => (1, 1),
            LayoutType::Void => (0, 1),
            LayoutType::Custom(_) => (8, 8), // Assume pointer size for custom types
            LayoutType::Pointer(_) => (8, 8),
            LayoutType::Reference(_) => (8, 8),
            LayoutType::Template(_, _) => (8, 8), // Assume pointer size for template types
        }
    }

    fn from_parser_type(data_type: &DataType) -> Self {
        match data_type {
            DataType::Int => LayoutType::Int,
            DataType::Float => LayoutType::Float,
            DataType::Double => LayoutType::Double,
            DataType::Char => LayoutType::Char,
            DataType::Bool => LayoutType::Bool,
            DataType::Void => LayoutType::Void,
            DataType::Custom(name) => LayoutType::Custom(name.to_string()),
            DataType::Pointer(inner) => {
                LayoutType::Pointer(Box::new(LayoutType::from_parser_type(inner)))
            }
            DataType::Reference(inner) => {
                LayoutType::Reference(Box::new(LayoutType::from_parser_type(inner)))
            }
            DataType::Template(name, args) => LayoutType::Template(
                name.to_string(),
                args.iter().map(LayoutType::from_parser_type).collect(),
            ),
        }
    }
}

pub fn compute_layouts(declarations: &[LayoutDeclaration]) -> HashMap<String, MemoryLayout> {
    let mut layouts = HashMap::new();

    for decl in declarations {
        let layout = compute_type_layout(&decl.sections);
        layouts.insert(decl.name.clone(), layout);
    }

    layouts
}

fn compute_type_layout(sections: &[LayoutSection]) -> MemoryLayout {
    let mut size = 0;
    let mut alignment = 1;
    let mut member_offsets = Vec::new();

    // Process each section
    for section in sections {
        for member in &section.members {
            let (member_size, member_alignment) = member.data_type.size_and_alignment();

            // Align the current size to the member's alignment
            size = (size + member_alignment - 1) & !(member_alignment - 1);

            // Record the member's offset
            member_offsets.push((member.name.clone(), size));

            // Update size and alignment
            size += member_size;
            alignment = alignment.max(member_alignment);
        }
    }

    // Align the final size to the type's alignment
    size = (size + alignment - 1) & !(alignment - 1);

    MemoryLayout {
        size,
        alignment,
        members: member_offsets,
    }
}

pub fn from_parser_declarations(declarations: &[Declaration]) -> Vec<LayoutDeclaration> {
    declarations
        .iter()
        .map(|decl| match decl {
            Declaration::Struct { name, members, .. }
            | Declaration::Class { name, members, .. } => LayoutDeclaration {
                name: name.to_string(),
                sections: members
                    .iter()
                    .map(|section| LayoutSection {
                        members: section
                            .members
                            .iter()
                            .filter_map(|member| match member {
                                Member::Data(data) => Some(LayoutMember {
                                    name: data.name.to_string(),
                                    data_type: LayoutType::from_parser_type(&data.data_type),
                                }),
                                Member::Function(_) => None,
                            })
                            .collect(),
                    })
                    .collect(),
            },
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_declaration() -> LayoutDeclaration {
        LayoutDeclaration {
            name: "SimpleStruct".to_string(),
            sections: vec![LayoutSection {
                members: vec![
                    LayoutMember {
                        name: "x".to_string(),
                        data_type: LayoutType::Int,
                    },
                    LayoutMember {
                        name: "y".to_string(),
                        data_type: LayoutType::Float,
                    },
                    LayoutMember {
                        name: "z".to_string(),
                        data_type: LayoutType::Double,
                    },
                ],
            }],
        }
    }

    #[test]
    fn test_simple_struct_layout() {
        let declarations = vec![create_test_declaration()];
        let layouts = compute_layouts(&declarations);

        let layout = layouts.get("SimpleStruct").unwrap();
        assert_eq!(layout.size, 16);
        assert_eq!(layout.alignment, 8);
        assert_eq!(layout.members.len(), 3);

        // Check member offsets
        let offsets: HashMap<_, _> = layout.members.iter().cloned().collect();
        assert_eq!(offsets.get("x"), Some(&0));
        assert_eq!(offsets.get("y"), Some(&4));
        assert_eq!(offsets.get("z"), Some(&8));
    }

    #[test]
    fn test_class_with_inheritance() {
        let declarations = vec![
            LayoutDeclaration {
                name: "Base".to_string(),
                sections: vec![LayoutSection {
                    members: vec![LayoutMember {
                        name: "base_member".to_string(),
                        data_type: LayoutType::Int,
                    }],
                }],
            },
            LayoutDeclaration {
                name: "Derived".to_string(),
                sections: vec![LayoutSection {
                    members: vec![LayoutMember {
                        name: "derived_member".to_string(),
                        data_type: LayoutType::Double,
                    }],
                }],
            },
        ];

        let layouts = compute_layouts(&declarations);

        let base_layout = layouts.get("Base").unwrap();
        assert_eq!(base_layout.size, 4);
        assert_eq!(base_layout.alignment, 4);

        let derived_layout = layouts.get("Derived").unwrap();
        assert_eq!(derived_layout.size, 16);
        assert_eq!(derived_layout.alignment, 8);
    }
}
