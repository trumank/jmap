use patternsleuth_resolvers::unreal::engine_version::EngineVersion;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Structs(pub Vec<StructInfo>);

#[derive(Serialize, Deserialize)]
pub struct StructInfo {
    pub name: String,
    pub size: u64,
    pub members: Vec<StructMember>,
}

#[derive(Serialize, Deserialize)]
pub struct StructMember {
    pub name: String,
    pub offset: u64,
    pub size: u64,
    //pub type_name: String,
}

pub fn get_struct_info_for_version(version: &EngineVersion) -> Option<Structs> {
    let json = match (version.major, version.minor) {
        (4, 25) => include_str!("../structs/struct_info_425.json"),
        (4, 27) => include_str!("../structs/struct_info_427.json"),
        (5, 0) => include_str!("../structs/struct_info_500.json"),
        (5, 1) => include_str!("../structs/struct_info_501.json"),
        (5, 2) => include_str!("../structs/struct_info_502.json"),
        (5, 3) => include_str!("../structs/struct_info_503.json"),
        (5, 4) => include_str!("../structs/struct_info_504.json"),
        (5, 5) => include_str!("../structs/struct_info_505.json"),
        _ => return None,
    };
    Some(serde_json::from_str(json).unwrap())
}
