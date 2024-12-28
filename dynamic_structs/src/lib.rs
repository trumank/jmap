use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct VTableEntry {
    pub index: usize,
    pub name: String,
    pub return_type: String,
    pub arguments: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ClassMember {
    pub name: String,
    pub type_name: String,
    pub offset: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BaseClass {
    pub name: String,
    pub offset: u32,
    pub virtual_base: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ClassInfo {
    pub name: String,
    pub kind: String, // "class", "struct", or "interface"
    pub base_classes: Vec<BaseClass>,
    pub members: Vec<ClassMember>,
    pub vtable: Vec<VTableEntry>,
    pub size: u32, // Class size if available
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OutputData {
    pub classes: Vec<ClassInfo>,
}
