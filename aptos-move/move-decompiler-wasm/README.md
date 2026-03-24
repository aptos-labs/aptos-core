# Move Decompiler WASM

**WebAssembly bindings for Move bytecode decompilation and analysis.**

This standalone library provides browser and Node.js compatible tools for working with Move bytecode:
- 🔍 **Decompile** bytecode back to Move source code
- 📋 **Disassemble** bytecode to human-readable assembly
- 📊 **Extract metadata** (functions, structs, dependencies)
- ✅ **Verify** bytecode integrity

## Why This Crate?

This is a **minimal, focused WASM library** that:
- ✅ Has no native-only dependencies (crypto, networking, filesystem)
- ✅ Builds successfully for `wasm32-unknown-unknown`
- ✅ Works in browsers and Node.js
- ✅ Small binary size (~2-5MB optimized)
- ✅ Zero-copy operations where possible

Perfect for:
- Web-based Move explorers
- Browser-based bytecode analysis tools
- Educational Move playgrounds
- On-chain package verification tools

## Quick Start

### Build

```bash
# Install wasm-pack if needed
cargo install wasm-pack

# Build for web (browser)
cd aptos-move/move-decompiler-wasm
wasm-pack build --target web --out-dir pkg

# Or build for Node.js
wasm-pack build --target nodejs --out-dir pkg-node

# Or build for bundlers (webpack, etc.)
wasm-pack build --target bundler --out-dir pkg-bundler
```

### Alternative: Build with cargo directly

```bash
# Set getrandom backend for WASM
export RUSTFLAGS="--cfg getrandom_backend=\"wasm_js\""

# Build
cargo build --target wasm32-unknown-unknown --release

# Output: target/wasm32-unknown-unknown/release/move_decompiler_wasm.wasm
```

### Use in Browser

```html
<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <title>Move Decompiler Demo</title>
</head>
<body>
    <h1>Move Bytecode Decompiler</h1>
    <input type="file" id="bytecode-file" accept=".mv">
    <pre id="output"></pre>

    <script type="module">
        import init, {
            decompile_module,
            disassemble_module,
            get_module_metadata,
            get_version_info
        } from './pkg/move_decompiler_wasm.js';

        // Initialize WASM
        await init();
        console.log('WASM loaded:', get_version_info());

        // Handle file upload
        document.getElementById('bytecode-file').addEventListener('change', async (e) => {
            const file = e.target.files[0];
            const bytes = new Uint8Array(await file.arrayBuffer());

            try {
                // Get metadata
                const metadata = get_module_metadata(bytes);
                console.log('Module:', metadata.name);
                console.log('Functions:', metadata.function_count);
                console.log(metadata.to_json());

                // Decompile
                const source = decompile_module(bytes);
                document.getElementById('output').textContent = source;

                // Or disassemble
                // const assembly = disassemble_module(bytes);
                // document.getElementById('output').textContent = assembly;
            } catch (err) {
                console.error('Error:', err);
                document.getElementById('output').textContent = 'Error: ' + err;
            }
        });
    </script>
</body>
</html>
```

### Use in Node.js

```javascript
// build-wasm-node.mjs
import { readFile } from 'fs/promises';
import init, {
    decompile_module,
    get_module_metadata,
    verify_module
} from './pkg-node/move_decompiler_wasm.js';

// Initialize
await init();

// Load bytecode
const bytecode = await readFile('example.mv');

// Verify
try {
    verify_module(bytecode);
    console.log('✓ Bytecode is valid');
} catch (e) {
    console.error('✗ Invalid bytecode:', e);
    process.exit(1);
}

// Get metadata
const metadata = get_module_metadata(bytecode);
console.log('Module:', metadata.name);
console.log('Address:', metadata.address);
console.log('Functions:', metadata.function_count);
console.log('Structs:', metadata.struct_count);

// Decompile
const source = decompile_module(bytecode);
console.log('\nDecompiled source:');
console.log(source);
```

Run with:
```bash
node build-wasm-node.mjs
```

### Use with TypeScript

TypeScript definitions are automatically generated:

```typescript
import init, {
    decompile_module,
    BytecodeMetadata
} from './pkg/move_decompiler_wasm';

await init();

async function analyzeModule(bytecode: Uint8Array): Promise<void> {
    const metadata: BytecodeMetadata = get_module_metadata(bytecode);
    const source: string = decompile_module(bytecode);

    console.log(`Module: ${metadata.name}`);
    console.log(`Source:\n${source}`);
}
```

## API Reference

### Functions

#### `decompile_module(bytecode: Uint8Array): string`
Decompile Move module bytecode to Move source code.

**Throws:** If bytecode is invalid or decompilation fails.

#### `decompile_script(bytecode: Uint8Array): string`
Decompile Move script bytecode to Move source code.

#### `disassemble_module(bytecode: Uint8Array): string`
Disassemble Move module bytecode to human-readable assembly format.

#### `disassemble_script(bytecode: Uint8Array): string`
Disassemble Move script bytecode to assembly format.

#### `get_module_metadata(bytecode: Uint8Array): BytecodeMetadata`
Extract metadata from module bytecode without full decompilation.

Returns:
- `name: string` - Module name
- `address: string | undefined` - Module address
- `version: number` - Bytecode version
- `function_count: number` - Number of functions
- `struct_count: number` - Number of structs
- `dependencies: string[]` - Module dependencies

#### `verify_module(bytecode: Uint8Array): boolean`
Verify module bytecode integrity (basic structural validation).

**Throws:** If bytecode is invalid.

#### `verify_script(bytecode: Uint8Array): boolean`
Verify script bytecode integrity.

#### `get_version_info(): string`
Get library version information as JSON string.

#### `init_panic_hook(): void`
Initialize panic hook for better error messages in browser console.

## Building & Optimization

### Development Build

```bash
wasm-pack build --dev --target web
```

### Production Build (Optimized)

```bash
# Build with optimizations
wasm-pack build --release --target web

# Further optimize with wasm-opt (from binaryen)
wasm-opt -Oz --enable-mutable-globals \
  pkg/move_decompiler_wasm_bg.wasm \
  -o pkg/move_decompiler_wasm_bg.wasm

# Check size
ls -lh pkg/move_decompiler_wasm_bg.wasm
```

### Size Optimization Tips

1. **Use release mode:** Always use `--release` for production
2. **wasm-opt:** Run `wasm-opt -Oz` for maximum compression
3. **gzip/brotli:** Serve with compression (50-70% reduction)
4. **Code splitting:** Load only needed functions

Expected sizes:
- Debug build: ~10-15 MB
- Release build: ~3-5 MB
- Release + wasm-opt: ~2-3 MB
- With gzip: ~800KB-1.5MB

## Testing

### Rust Tests

```bash
cargo test
```

### WASM Tests

```bash
wasm-pack test --headless --chrome
wasm-pack test --headless --firefox
```

### Manual Testing

```bash
# Build
wasm-pack build --target web

# Serve locally
python3 -m http.server 8000

# Open http://localhost:8000 and test with browser console
```

## Example: Complete Web App

See [examples/web-decompiler](./examples/web-decompiler/) for a complete example web application.

## Dependencies

This crate has **minimal dependencies**:
- `move-binary-format` - Bytecode format definitions
- `move-core-types` - Core Move types
- `move-decompiler` - Decompilation logic
- `wasm-bindgen` - Rust ↔ JavaScript bindings
- `getrandom` (with "js" feature) - Random number generation for WASM

**No dependencies on:**
- ❌ `aptos-crypto` (native crypto)
- ❌ `aptos-rest-client` (networking)
- ❌ `tokio` (async runtime)
- ❌ `tempfile` (filesystem)

This means it **will build successfully** for WASM!

## Troubleshooting

### Build fails with "getrandom" error

**Solution:** Set the RUSTFLAGS environment variable:
```bash
export RUSTFLAGS="--cfg getrandom_backend=\"wasm_js\""
cargo build --target wasm32-unknown-unknown
```

Or use `wasm-pack` which handles this automatically.

### Binary size too large

**Solutions:**
1. Use `--release` mode
2. Run `wasm-opt -Oz`
3. Enable LTO in `Cargo.toml`:
   ```toml
   [profile.release]
   lto = true
   opt-level = "z"
   ```

### Errors in browser console

**Solution:** Call `init_panic_hook()` after initializing WASM:
```javascript
await init();
init_panic_hook(); // Better error messages
```

## Related Projects

- [aptos-move-cli](../cli/) - Native Move CLI tools
- [aptos-dynamic-transaction-composer](../script-composer/) - Script composition in WASM
- [Move Language](https://github.com/move-language/move) - Core Move implementation

## License

Apache-2.0

## Contributing

This is part of the Aptos Core repository. See the main [CONTRIBUTING.md](../../CONTRIBUTING.md).

## Status

**Current:** ✅ Functional proof-of-concept
**Next:** Production hardening, npm package, comprehensive tests

---

**Questions?** See the [WASM Evaluation](../cli/WASM_EVALUATION_README.md) for full feasibility analysis.
