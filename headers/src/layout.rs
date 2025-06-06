use crate::parser::{AccessSection, DataType, Declaration, Member};
use std::backtrace::Backtrace;
use std::collections::HashMap;
use std::fmt;

#[macro_export]
macro_rules! layout_error {
    ($variant:ident, $($arg:expr),*) => {{
        let backtrace = Backtrace::capture();
        $crate::layout::LayoutError {
            kind: $crate::layout::LayoutErrorKind::$variant {
                message: format!($($arg),*),
            },
            backtrace,
        }
    }};
}

pub enum LayoutErrorKind {
    UndefinedType { message: String },
    TemplateType { message: String },
}

pub struct LayoutError {
    pub kind: LayoutErrorKind,
    pub backtrace: Backtrace,
}

impl fmt::Debug for LayoutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl fmt::Display for LayoutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            LayoutErrorKind::UndefinedType { message } => {
                write!(f, "Undefined type: {message}")?;
            }
            LayoutErrorKind::TemplateType { message } => {
                write!(f, "Cannot compute layout for template type: {message}")?;
            }
        }
        if self.backtrace.status() == std::backtrace::BacktraceStatus::Captured {
            write!(f, "\nUndefined type: {}", self.backtrace)?;
        }
        Ok(())
    }
}

impl std::error::Error for LayoutError {}

impl LayoutError {
    pub fn undefined_type(message: impl Into<String>) -> Self {
        Self {
            kind: LayoutErrorKind::UndefinedType {
                message: message.into(),
            },
            backtrace: Backtrace::capture(),
        }
    }

    pub fn template_type(message: impl Into<String>) -> Self {
        Self {
            kind: LayoutErrorKind::TemplateType {
                message: message.into(),
            },
            backtrace: Backtrace::capture(),
        }
    }
}

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
    Array(Box<LayoutType>, usize), // (element_type, length)
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
    fn from_parser_type(data_type: &DataType) -> Result<Self, LayoutError> {
        match data_type {
            DataType::Int => Ok(LayoutType::Int),
            DataType::Float => Ok(LayoutType::Float),
            DataType::Double => Ok(LayoutType::Double),
            DataType::Char => Ok(LayoutType::Char),
            DataType::Bool => Ok(LayoutType::Bool),
            DataType::Void => Ok(LayoutType::Void),
            DataType::Custom(name) => Ok(LayoutType::Custom(name.to_string())),
            DataType::Pointer(inner) => {
                // For pointers, we don't need to validate the inner type
                Ok(LayoutType::Pointer(Box::new(LayoutType::from_parser_type(
                    inner,
                )?)))
            }
            DataType::Reference(inner) => {
                // For references, we don't need to validate the inner type
                Ok(LayoutType::Reference(Box::new(
                    LayoutType::from_parser_type(inner)?,
                )))
            }
            DataType::Template(name, _) => Err(LayoutError::template_type(name.to_string())),
            DataType::Array(inner, size) => Ok(LayoutType::Array(
                Box::new(LayoutType::from_parser_type(inner)?),
                *size,
            )),
        }
    }

    fn size_and_alignment(
        &self,
        layouts: &HashMap<String, MemoryLayout>,
    ) -> Result<(usize, usize), LayoutError> {
        match self {
            LayoutType::Int => Ok((4, 4)),
            LayoutType::Float => Ok((4, 4)),
            LayoutType::Double => Ok((8, 8)),
            LayoutType::Char => Ok((1, 1)),
            LayoutType::Bool => Ok((1, 1)),
            LayoutType::Void => Ok((0, 1)),
            LayoutType::Custom(name) => layouts
                .get(name)
                .map(|layout| Ok((layout.size, layout.alignment)))
                .unwrap_or(Err(LayoutError::undefined_type(name.clone()))),
            LayoutType::Pointer(_) => Ok((8, 8)),
            LayoutType::Reference(_) => Ok((8, 8)),
            LayoutType::Template(name, _) => Err(LayoutError::template_type(name.clone())),
            LayoutType::Array(inner, size) => {
                let (element_size, element_alignment) = inner.size_and_alignment(layouts)?;
                Ok((element_size * size, element_alignment))
            }
        }
    }
}

pub fn compute_layouts(
    declarations: &[LayoutDeclaration],
) -> Result<HashMap<String, MemoryLayout>, LayoutError> {
    let mut layouts = HashMap::new();

    // First pass: compute sizes and alignments for all types
    for decl in declarations {
        let layout = compute_type_layout(&decl.sections, &layouts)?;
        layouts.insert(decl.name.clone(), layout);
    }

    Ok(layouts)
}

fn compute_type_layout(
    sections: &[LayoutSection],
    layouts: &HashMap<String, MemoryLayout>,
) -> Result<MemoryLayout, LayoutError> {
    let mut size = 0;
    let mut alignment = 1;
    let mut member_offsets = Vec::new();

    // Process each section
    for section in sections {
        for member in &section.members {
            let (member_size, member_alignment) = member.data_type.size_and_alignment(layouts)?;

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

    Ok(MemoryLayout {
        size,
        alignment,
        members: member_offsets,
    })
}

pub fn from_parser_declarations(
    declarations: &[Declaration],
) -> Result<Vec<LayoutDeclaration>, LayoutError> {
    declarations
        .iter()
        .map(|decl| match decl {
            Declaration::Struct { name, members, .. }
            | Declaration::Class { name, members, .. } => {
                let mut sections = vec![];
                for section in members {
                    let mut members = vec![];
                    for member in &section.members {
                        match member {
                            Member::Data(data) => members.push(LayoutMember {
                                name: data.name.to_string(),
                                data_type: LayoutType::from_parser_type(&data.data_type)?,
                            }),
                            Member::Function(_) => {}
                        }
                    }
                    sections.push(LayoutSection { members });
                }
                Ok(LayoutDeclaration {
                    name: name.to_string(),
                    sections,
                })
            }
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
        let layouts = compute_layouts(&declarations).unwrap();

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
    fn test_custom_type_layout() {
        let declarations = vec![
            LayoutDeclaration {
                name: "Point".to_string(),
                sections: vec![LayoutSection {
                    members: vec![
                        LayoutMember {
                            name: "x".to_string(),
                            data_type: LayoutType::Float,
                        },
                        LayoutMember {
                            name: "y".to_string(),
                            data_type: LayoutType::Float,
                        },
                    ],
                }],
            },
            LayoutDeclaration {
                name: "Line".to_string(),
                sections: vec![LayoutSection {
                    members: vec![
                        LayoutMember {
                            name: "start".to_string(),
                            data_type: LayoutType::Custom("Point".to_string()),
                        },
                        LayoutMember {
                            name: "end".to_string(),
                            data_type: LayoutType::Custom("Point".to_string()),
                        },
                    ],
                }],
            },
        ];

        let layouts = compute_layouts(&declarations).unwrap();

        let point_layout = layouts.get("Point").unwrap();
        assert_eq!(point_layout.size, 8);
        assert_eq!(point_layout.alignment, 4);

        let line_layout = layouts.get("Line").unwrap();
        assert_eq!(line_layout.size, 16);
        assert_eq!(line_layout.alignment, 4);

        // Check member offsets
        let offsets: HashMap<_, _> = line_layout.members.iter().cloned().collect();
        assert_eq!(offsets.get("start"), Some(&0));
        assert_eq!(offsets.get("end"), Some(&8));
    }

    #[test]
    fn test_array_type_layout() {
        let declarations = vec![LayoutDeclaration {
            name: "ArrayStruct".to_string(),
            sections: vec![LayoutSection {
                members: vec![
                    LayoutMember {
                        name: "ints".to_string(),
                        data_type: LayoutType::Array(Box::new(LayoutType::Int), 5),
                    },
                    LayoutMember {
                        name: "doubles".to_string(),
                        data_type: LayoutType::Array(Box::new(LayoutType::Double), 3),
                    },
                ],
            }],
        }];

        let layouts = compute_layouts(&declarations).unwrap();
        let layout = layouts.get("ArrayStruct").unwrap();

        // Array of 5 ints (4 bytes each) = 20 bytes
        // Array of 3 doubles (8 bytes each) = 24 bytes
        // Total size should be 48 bytes (aligned to 8 bytes)
        assert_eq!(layout.size, 48);
        assert_eq!(layout.alignment, 8);

        // Check member offsets
        let offsets: HashMap<_, _> = layout.members.iter().cloned().collect();
        assert_eq!(offsets.get("ints"), Some(&0));
        assert_eq!(offsets.get("doubles"), Some(&24));
    }

    #[test]
    fn test_error_backtrace() {
        let declarations = vec![LayoutDeclaration {
            name: "ErrorStruct".to_string(),
            sections: vec![LayoutSection {
                members: vec![LayoutMember {
                    name: "value".to_string(),
                    data_type: LayoutType::Custom("UndefinedType".to_string()),
                }],
            }],
        }];

        let result = compute_layouts(&declarations);
        assert!(matches!(
            result,
            Err(LayoutError {
                kind: LayoutErrorKind::UndefinedType { .. },
                ..
            })
        ));

        if let Err(LayoutError { backtrace, .. }) = result {
            let backtrace_str = format!("{}", backtrace);
            assert!(!backtrace_str.is_empty());
            assert!(backtrace_str.contains("test_error_backtrace"));
        }
    }

    #[test]
    fn test_template_type_error() {
        let declarations = vec![LayoutDeclaration {
            name: "TemplateStruct".to_string(),
            sections: vec![LayoutSection {
                members: vec![LayoutMember {
                    name: "value".to_string(),
                    data_type: LayoutType::Template("Vector".to_string(), vec![LayoutType::Int]),
                }],
            }],
        }];

        let result = compute_layouts(&declarations);
        assert!(matches!(
            result,
            Err(LayoutError {
                kind: LayoutErrorKind::TemplateType { .. },
                ..
            })
        ));

        if let Err(LayoutError { backtrace, .. }) = result {
            let backtrace_str = format!("{}", backtrace);
            assert!(!backtrace_str.is_empty());
            assert!(backtrace_str.contains("test_template_type_error"));
        }
    }

    #[test]
    fn test_pointer_to_undefined_type() {
        let declarations = vec![LayoutDeclaration {
            name: "PointerStruct".to_string(),
            sections: vec![LayoutSection {
                members: vec![LayoutMember {
                    name: "ptr".to_string(),
                    data_type: LayoutType::Pointer(Box::new(LayoutType::Custom(
                        "UndefinedType".to_string(),
                    ))),
                }],
            }],
        }];

        let result = compute_layouts(&declarations);
        assert!(result.is_ok()); // Should succeed since pointers don't need to validate their pointee type
    }
}
