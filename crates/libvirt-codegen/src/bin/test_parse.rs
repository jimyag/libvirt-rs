// Test script to check protocol parsing and code generation
fn main() {
    let content = std::fs::read_to_string("proto/remote_protocol.x").unwrap();
    match libvirt_codegen::parser::parse_protocol(&content) {
        Ok(protocol) => {
            println!("Parse successful!");
            println!("Constants: {}", protocol.constants.len());
            println!("Types: {}", protocol.types.len());
            println!("Procedures: {}", protocol.procedures.len());

            // Print first 5 procedures with their args/ret
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

            // Write to file for inspection
            std::fs::write("tmp/generated.rs", &code).unwrap();
            println!("Written to tmp/generated.rs");

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
