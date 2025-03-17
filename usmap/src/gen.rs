//#[derive(Debug, Clone, Serialize)]
//pub struct FullStruct {
//    name: String,
//    package: String,
//    super_struct: Option<String>,
//    properties: Vec<Property>,
//}
//impl FullStruct {
//    fn gen_header(&self) -> String {
//        use std::fmt::Write;
//
//        fn prefix_name(name: impl AsRef<str>) -> String {
//            // TODO U for UObject A for Actor
//            format!("U{}", name.as_ref())
//        }
//
//        let mut buf = String::new();
//        writeln!(&mut buf, "#pragma once").unwrap();
//        writeln!(&mut buf, "#include <CoreMinimal.h>").unwrap();
//
//        // TODO
//        let super_struct = if let Some(super_struct) = &self.super_struct {
//            format!(" : {}", prefix_name(super_struct))
//        } else {
//            "".to_string()
//        };
//
//        writeln!(
//            &mut buf,
//            "class {}{super_struct} {{",
//            prefix_name(&self.name)
//        )
//        .unwrap();
//
//        for prop in &self.properties {
//            writeln!(&mut buf, "    UPROPERTY(BlueprintReadWrite), EditEverywhere, Transient, meta=(AllowPrivateAcess=true)").unwrap();
//            match &prop.inner {
//                //PropertyInner::Byte => todo!(),
//                //PropertyInner::Bool => todo!(),
//                //PropertyInner::Int => todo!(),
//                //PropertyInner::Float => todo!(),
//                PropertyInner::Object => writeln!(&mut buf, "    UObject* {}", prop.name).unwrap(),
//                //PropertyInner::Name => todo!(),
//                //PropertyInner::Delegate => todo!(),
//                //PropertyInner::Double => todo!(),
//                //PropertyInner::Array { inner } => todo!(),
//                //PropertyInner::Struct { name } => todo!(),
//                //PropertyInner::Str => todo!(),
//                //PropertyInner::Text => todo!(),
//                //PropertyInner::Interface => todo!(),
//                //PropertyInner::MulticastDelegate => todo!(),
//                //PropertyInner::WeakObject => todo!(),
//                //PropertyInner::LazyObject => todo!(),
//                //PropertyInner::AssetObject => todo!(),
//                //PropertyInner::SoftObject => todo!(),
//                //PropertyInner::UInt64 => todo!(),
//                //PropertyInner::UInt32 => todo!(),
//                //PropertyInner::UInt16 => todo!(),
//                //PropertyInner::Int64 => todo!(),
//                //PropertyInner::Int16 => todo!(),
//                //PropertyInner::Int8 => todo!(),
//                //PropertyInner::Map { key, value } => todo!(),
//                //PropertyInner::Set { key } => todo!(),
//                //PropertyInner::Enum { inner, name } => todo!(),
//                //PropertyInner::FieldPath => todo!(),
//                //PropertyInner::EnumAsByte => todo!(),
//                _ => writeln!(&mut buf, "TODO {:?}", prop).unwrap(),
//            }
//            writeln!(&mut buf).unwrap();
//        }
//
//        writeln!(&mut buf, "}}").unwrap();
//
//        buf
//    }
//}
//
//impl Usmap {
//    pub fn full_structs(&self) -> Vec<FullStruct> {
//        self.structs
//            .iter()
//            .zip(self.ppth.as_ref().unwrap().structs.iter())
//            .map(|(s, path)| FullStruct {
//                name: s.name.clone(),
//                package: path.clone(),
//                super_struct: s.super_struct.clone(),
//                properties: s.properties.clone(),
//            })
//            .collect()
//    }
//    pub fn gen(&self) {
//        for full in self.full_structs() {
//            if full.name == "GeneratedMission" {
//                dbg!(&full);
//                println!("{}", full.gen_header());
//            }
//        }
//    }
//}
