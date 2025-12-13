// Test script to check protocol parsing
fn main() {
    let content = std::fs::read_to_string("proto/remote_protocol.x").unwrap();
    match libvirt_codegen::parser::parse_protocol(&content) {
        Ok(protocol) => {
            println!("Parse successful!");
            println!("Constants: {}", protocol.constants.len());
            println!("Types: {}", protocol.types.len());
            println!("Procedures: {}", protocol.procedures.len());

            // Print first 10 constants
            println!("\nFirst 10 constants:");
            for c in protocol.constants.iter().take(10) {
                println!("  {}", c.name);
            }

            // Print first 10 types
            println!("\nFirst 10 types:");
            for t in protocol.types.iter().take(10) {
                let name = match t {
                    libvirt_codegen::ast::TypeDef::Struct(s) => format!("struct {}", s.name),
                    libvirt_codegen::ast::TypeDef::Enum(e) => format!("enum {}", e.name),
                    libvirt_codegen::ast::TypeDef::Union(u) => format!("union {}", u.name),
                    libvirt_codegen::ast::TypeDef::Typedef(t) => format!("typedef {}", t.name),
                };
                println!("  {}", name);
            }
        }
        Err(e) => {
            // Show first 1000 chars of error
            let preview: String = e.chars().take(1000).collect();
            eprintln!("Parse error:\n{}", preview);
        }
    }
}
