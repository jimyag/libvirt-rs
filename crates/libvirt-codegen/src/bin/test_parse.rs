// Test script to check protocol parsing and code generation
fn main() {
    // Proto file is now in crates/libvirt/proto/
    let proto_path = "../libvirt/proto/remote_protocol.x";
    let content = std::fs::read_to_string(proto_path)
        .unwrap_or_else(|_| panic!("Failed to read {}", proto_path));

    match libvirt_codegen::parser::parse_protocol(&content) {
        Ok(protocol) => {
            println!("Parse successful!");
            println!("Constants: {}", protocol.constants.len());
            println!("Types: {}", protocol.types.len());
            println!("Procedures: {}", protocol.procedures.len());

            // Print first 10 procedures with their args/ret
            println!("\nFirst 10 procedures:");
            for p in protocol.procedures.iter().take(10) {
                println!(
                    "  {} = {} (args: {:?}, ret: {:?})",
                    p.name, p.number, p.args, p.ret
                );
            }

            // Test code generation
            println!("\nGenerating code...");
            let code = libvirt_codegen::generate(&protocol);
            println!("Generated {} bytes of code", code.len());

            // Show first 2000 chars
            println!("\nFirst 2000 chars of generated code:");
            println!("{}", &code[..code.len().min(2000)]);
        }
        Err(e) => {
            // Show first 1000 chars of error
            let preview: String = e.chars().take(1000).collect();
            eprintln!("Parse error:\n{}", preview);
        }
    }
}
