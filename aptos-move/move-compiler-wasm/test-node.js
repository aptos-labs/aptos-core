// Test if WASM compiler works in Node.js
import { readFile } from 'fs/promises';
import init, { compile_module } from './pkg/move_compiler_wasm.js';

const wasmBuffer = await readFile('./pkg/move_compiler_wasm_bg.wasm');
await init(wasmBuffer);

const source = `
module 0x1::Test {
    public fun hello(): u64 {
        42
    }
}
`;

console.log('Testing Move compiler in Node.js...\n');

try {
    console.log('Compiling...');
    const result = compile_module(source, "0x1", "Test");
    console.log('\nResult:');
    console.log('  Success:', result.success);
    console.log('  Bytecode length:', result.bytecode.length);

    const errors = JSON.parse(result.errors);
    if (errors.length > 0) {
        console.log('\nErrors:');
        errors.forEach(e => console.log('  -', e));
    }

    if (result.success && result.bytecode.length > 0) {
        console.log('\nBytecode (first 100 bytes):');
        console.log('  ', Array.from(result.bytecode.slice(0, 100))
            .map(b => b.toString(16).padStart(2, '0'))
            .join(' '));
    }
} catch (e) {
    console.error('\nException:', e.message);
    console.error(e.stack);
}
