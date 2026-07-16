# Prebuilt WASM (bytecode v10)

This folder contains a **prebuilt, ready-to-download** WebAssembly build of the
`move-decompiler-wasm` crate, targeting Move **bytecode version 10**. It now
also bundles the Move **compiler** (`compile_module`/`compile_script`), so the
`.wasm` is substantially larger than a decompiler-only build.

It is produced by [`../build_wasm.sh`](../build_wasm.sh) (which wraps
`wasm-pack build --target web`). Regenerate it any time with:

```bash
cd aptos-move/move-decompiler-wasm
./build_wasm.sh
```

## Contents

| File | Description |
| ---- | ----------- |
| `move_decompiler_wasm_bg.wasm` | The compiled WebAssembly module (web target) |
| `move_decompiler_wasm.js` | wasm-bindgen JS glue / loader (ES module) |
| `move_decompiler_wasm.d.ts` | TypeScript type definitions |
| `move_decompiler_wasm_bg.wasm.d.ts` | Types for the raw wasm exports |
| `samples/sample_v10_module.mv` | A Move module serialized at bytecode v10 |
| `samples/sample_v10_script.mv` | A Move script serialized at bytecode v10 |

## Verified against bytecode v10

This build was verified end-to-end (see `../examples/verify_node.mjs`) against
real bytecode-v10 module **and script** artifacts, including compiling Move
source to v10 bytecode:

- `get_version_info().max_bytecode_version == 10`
- `verify_module` / `verify_script` accept v10 bytecode
- `disassemble_module` / `disassemble_script` emit `// Bytecode version v10`
- `decompile_module` / `decompile_script` succeed
- `get_module_metadata(...).version == 10`
- `compile_module` / `compile_script` turn Move source into v10 bytecode, which
  then round-trips through `verify_*` / `disassemble_*`

## Quick use (browser / web target)

```html
<script type="module">
  import init, {
    get_version_info,
    disassemble_script,
    decompile_script,
  } from "./move_decompiler_wasm.js";

  await init(); // loads move_decompiler_wasm_bg.wasm
  console.log(JSON.parse(get_version_info()).max_bytecode_version); // 10

  const resp = await fetch("./samples/sample_v10_script.mv");
  const bytes = new Uint8Array(await resp.arrayBuffer());
  console.log(disassemble_script(bytes));
  console.log(decompile_script(bytes));
</script>
```

Serve the folder (module scripts require http, not `file://`):

```bash
python3 -m http.server 8000   # then open http://localhost:8000
```

## Node.js

For Node.js, build the `nodejs` target instead (`build_wasm.sh` also produces
`../pkg-node/`) and see `../examples/verify_node.mjs` for a working example.
