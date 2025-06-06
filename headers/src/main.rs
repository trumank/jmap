mod layout;
mod parser;

use std::{env, fs};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let filename = env::args().nth(1).expect("Expected file argument");
    let src = fs::read_to_string(&filename).expect("Failed to read file");

    let declarations = parser::parse(&filename, &src);
    let layout_declarations = layout::from_parser_declarations(&declarations)?;
    let layouts = layout::compute_layouts(&layout_declarations)?;

    dbg!(&layouts);
    // Print memory layouts
    println!("\nMemory Layouts:");
    println!("==============");
    for (name, layout) in layouts.iter() {
        println!("\nType: {}", name);
        println!("Size: {} bytes", layout.size);
        println!("Alignment: {} bytes", layout.alignment);
        println!("Members:");
        for (member_name, offset) in &layout.members {
            println!("  {}: offset {} bytes", member_name, offset);
        }
    }
    Ok(())
}
