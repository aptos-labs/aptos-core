module aptos_std::wasm {
    use aptos_std::table::{Self, Table};

    struct WasmProgram has store {
        code: vector<u8>,
        globals: Table<vector<u8>, vector<u8>>,
    }

    /// Pack a value into the `Any` representation. Because Any can be stored and dropped, this is
    /// also required from `T`.
    public fun publish_code(code: vector<u8>): WasmProgram {
        let validated_code = Self::validate_and_annotate_wasm_bytecode(code);
        WasmProgram {
            code,
            globals: table::new(),
        }
    }

    native fun validate_and_annotate_wasm_bytecode(code: vector<u8>): vector<u8>;
    native fun execute_bytecode(program: &vector<u8>, globals: &Table<vector<u8>, vector<u8>>, args: vector<u8>, is_mutable: bool): vector<u8>;

    public fun execute_code(program: &WasmProgram, args: vector<u8>): vector<u8> {
        Self::execute_bytecode(&program.code, &program.globals, args, false)
    }

    public fun execute_code_mutable(program: &mut WasmProgram, args: vector<u8>): vector<u8> {
        Self::execute_bytecode(&program.code, &program.globals, args, true)
    }
}
