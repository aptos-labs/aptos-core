use move_smith::{utils::raw_to_module, CodeGenerator};
use std::io::{self, Read, Write};

fn main() -> io::Result<()> {
    let mut buffer = Vec::new();
    io::stdin().read_to_end(&mut buffer)?;

    let code = match raw_to_module(&buffer) {
        Ok(module) => module.emit_code(),
        Err(_) => panic!("Failed to parse raw bytes"),
    };

    io::stdout().write_all(code.as_bytes())
}
