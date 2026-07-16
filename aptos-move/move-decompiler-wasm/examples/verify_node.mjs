// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// End-to-end check that the compiled WASM works on real bytecode-v10 artifacts.
//
// Prereqs:
//   wasm-pack build --release --target nodejs --out-dir pkg-node
//   cargo run --example gen_samples
//
// Run:
//   node examples/verify_node.mjs

import { readFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const here = dirname(fileURLToPath(import.meta.url));
const crate = join(here, "..");

const wasm = await import(join(crate, "pkg-node", "move_decompiler_wasm.js"));

const {
  get_version_info,
  get_module_metadata,
  disassemble_module,
  decompile_module,
  verify_module,
  disassemble_script,
  decompile_script,
  verify_script,
} = wasm;

const info = JSON.parse(get_version_info());
console.log("version info:", info);
if (info.max_bytecode_version < 10) {
  throw new Error(`expected max_bytecode_version >= 10, got ${info.max_bytecode_version}`);
}

const moduleBytes = new Uint8Array(await readFile(join(crate, "samples", "sample_v10_module.mv")));
const scriptBytes = new Uint8Array(await readFile(join(crate, "samples", "sample_v10_script.mv")));

console.log("\n=== v10 MODULE ===");
if (!verify_module(moduleBytes)) throw new Error("module failed verification");
const meta = get_module_metadata(moduleBytes);
console.log("name:", meta.name, "| version:", meta.version, "| functions:", meta.functionCount);
if (meta.version !== 10) throw new Error(`expected module version 10, got ${meta.version}`);
console.log("disassembly (first line):", disassemble_module(moduleBytes).split("\n")[0]);
console.log("decompile ok, length:", decompile_module(moduleBytes).length);

console.log("\n=== v10 SCRIPT ===");
if (!verify_script(scriptBytes)) throw new Error("script failed verification");
const scriptAsm = disassemble_script(scriptBytes);
console.log("disassembly (first line):", scriptAsm.split("\n")[0]);
if (scriptAsm.length === 0) throw new Error("empty script disassembly");
const scriptSrc = decompile_script(scriptBytes);
console.log("decompile ok, length:", scriptSrc.length);

console.log("\nAll v10 module + script checks passed.");
