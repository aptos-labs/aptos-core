# Move Decompiler WASM

**WebAssembly bindings for Move bytecode decompilation and analysis.**

This standalone library provides browser and Node.js compatible tools for working with Move bytecode:
- 🛠️ **Compile** Move source code to bytecode (bytecode v10)
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

### Prebuilt WASM (bytecode v10)

A ready-to-download build lives in [`dist/`](./dist/) — grab
`dist/move_decompiler_wasm_bg.wasm` (plus the `.js`/`.d.ts` glue) directly. It is
verified end-to-end against real bytecode-v10 module **and** script artifacts
(`dist/samples/*.mv`). See [`dist/README.md`](./dist/README.md).

### One-command build (recommended)

```bash
cd aptos-move/move-decompiler-wasm
./build_wasm.sh          # builds pkg/ + pkg-node/, generates samples, refreshes dist/, verifies in node
```

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

#### `compile_module(source: string, named_addresses_json: string): Uint8Array`
Compile Move **module** source code to bytecode, serialized at bytecode
version 10. `named_addresses_json` is a JSON object mapping named addresses to
hex addresses (e.g. `{"my_addr":"0x42"}`), or `""` if the source only uses
numeric addresses. All source is read from memory — no filesystem is required,
which is what allows the Move compiler front-end to run under WASM.

**Throws:** compiler diagnostics if compilation fails or no module is produced.

#### `compile_script(source: string, named_addresses_json: string): Uint8Array`
Compile Move **script** source code (a `script { ... }` block) to bytecode at
bytecode version 10.

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

## Supported bytecode versions

This crate is built against the latest Aptos `move-binary-format` and accepts
every bytecode version up to and including **v10** (the current
`VERSION_MAX`/`VERSION_DEFAULT`). The reported `max_bytecode_version` is exposed
through `get_version_info()`.

## Scope

In addition to read-only tooling (decompile/disassemble/verify/metadata), this
crate now also bundles the Move **compiler** front-end, exposed via
`compile_module`/`compile_script`. The compiler reads all source from memory,
so it runs under `wasm32-unknown-unknown` without a filesystem. Because it
embeds the compiler pipeline, the WASM binary is substantially larger than the
decompiler-only build (tens of MB unoptimized; run `wasm-opt -Oz` and serve
compressed to reduce it).

## Testing

The crate ships with host-runnable unit tests that build real compiled modules
across all supported bytecode versions (including v10) and exercise
deserialization, disassembly, decompilation, and metadata extraction:

```bash
cd aptos-move/move-decompiler-wasm

# Run the native unit tests
cargo test

# Verify the crate also compiles to WebAssembly
cargo build --target wasm32-unknown-unknown
```

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

Core dependencies:
- `move-binary-format` - Bytecode format definitions
- `move-core-types` - Core Move types
- `move-decompiler` - Decompilation logic
- `move-asm` - Disassembler
- `move-compiler-v2` / `legacy-move-compiler` - Move compiler front-end (for
  `compile_module`/`compile_script`)
- `wasm-bindgen` - Rust ↔ JavaScript bindings
- `getrandom` (with "js" feature) - Random number generation for WASM

**No dependencies on:**
- ❌ `aptos-crypto` (native crypto)
- ❌ `aptos-rest-client` (networking)
- ❌ `tokio` (async runtime)

All of the above build for `wasm32-unknown-unknown`. The Move compiler
front-end normally reads sources from disk; this crate feeds it source text
in-memory (see the `sources_content` / VFS plumbing in `move-compiler-v2`,
`move-model`, and `legacy-move-compiler`) so it works without a filesystem.

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
