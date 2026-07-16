/* tslint:disable */
/* eslint-disable */

/**
 * Metadata extracted from Move bytecode
 */
export class BytecodeMetadata {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Get dependencies as JSON array string
     */
    getDependencies(): string;
    /**
     * Convert to JSON string
     */
    toJSON(): string;
    /**
     * Get module address
     */
    readonly address: string | undefined;
    /**
     * Get function count
     */
    readonly functionCount: number;
    /**
     * Get module name
     */
    readonly name: string;
    /**
     * Get struct count
     */
    readonly structCount: number;
    /**
     * Get bytecode version
     */
    readonly version: number;
}

/**
 * Decompile Move module bytecode to Move source code
 *
 * Takes raw bytecode bytes and returns decompiled Move source code.
 *
 * # Arguments
 * * `bytecode` - Raw bytes from a .mv file (compiled Move module)
 *
 * # Returns
 * Decompiled Move source code as a string
 *
 * # Errors
 * Returns error if:
 * - Bytecode is invalid or corrupted
 * - Bytecode format version is unsupported
 * - Decompilation fails
 *
 * # Example
 * ```javascript
 * const source = decompile_module(bytecodeBytes);
 * console.log(source);
 * // module 0x1::MyModule {
 * //     public fun hello(): u64 { 42 }
 * // }
 * ```
 */
export function decompile_module(bytecode: Uint8Array): string;

/**
 * Decompile Move script bytecode to Move source code
 *
 * # Arguments
 * * `bytecode` - Raw bytes from a compiled Move script
 *
 * # Returns
 * Decompiled Move script source code
 */
export function decompile_script(bytecode: Uint8Array): string;

/**
 * Disassemble Move module bytecode to human-readable assembly
 *
 * Produces a detailed assembly-like representation of the bytecode,
 * showing all instructions, function signatures, and struct layouts.
 *
 * # Arguments
 * * `bytecode` - Raw bytes from a .mv file
 *
 * # Returns
 * Disassembled bytecode as formatted text
 *
 * # Example
 * ```javascript
 * const assembly = disassemble_module(bytecodeBytes);
 * console.log(assembly);
 * // // Module: 0x1::MyModule
 * // // Function: hello
 * // public hello(): u64 {
 * //   L0: LdU64(42)
 * //   L1: Ret
 * // }
 * ```
 */
export function disassemble_module(bytecode: Uint8Array): string;

/**
 * Disassemble Move script bytecode
 *
 * # Arguments
 * * `bytecode` - Raw bytes from a compiled Move script
 *
 * # Returns
 * Disassembled script bytecode
 */
export function disassemble_script(bytecode: Uint8Array): string;

/**
 * Extract metadata from Move module bytecode
 *
 * Returns structured information about the module without full decompilation.
 *
 * # Arguments
 * * `bytecode` - Raw bytes from a .mv file
 *
 * # Returns
 * BytecodeMetadata object with module information
 *
 * # Example
 * ```javascript
 * const metadata = get_module_metadata(bytecodeBytes);
 * console.log(metadata.name); // "MyModule"
 * console.log(metadata.to_json()); // Full JSON representation
 * ```
 */
export function get_module_metadata(bytecode: Uint8Array): BytecodeMetadata;

/**
 * Get library version information
 *
 * Returns version and build information for the WASM library.
 *
 * # Returns
 * JSON string with version details
 */
export function get_version_info(): string;

/**
 * Initialize panic hook for better error messages in browser console
 *
 * Call this once when your WASM module loads to get better error messages.
 *
 * # Example
 * ```javascript
 * import init, { init_panic_hook } from './move_decompiler_wasm.js';
 *
 * await init();
 * init_panic_hook(); // Better error messages in browser
 * ```
 */
export function init_panic_hook(): void;

/**
 * Verify Move module bytecode integrity
 *
 * Performs basic bytecode verification without full VM validation.
 *
 * # Arguments
 * * `bytecode` - Raw bytes to verify
 *
 * # Returns
 * - `Ok(true)` if bytecode is valid
 * - `Err(...)` if bytecode is invalid
 *
 * # Example
 * ```javascript
 * try {
 *     verify_module(bytecodeBytes);
 *     console.log("✓ Bytecode is valid");
 * } catch (e) {
 *     console.error("✗ Invalid bytecode:", e);
 * }
 * ```
 */
export function verify_module(bytecode: Uint8Array): boolean;

/**
 * Verify Move script bytecode integrity
 */
export function verify_script(bytecode: Uint8Array): boolean;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_bytecodemetadata_free: (a: number, b: number) => void;
    readonly bytecodemetadata_address: (a: number) => [number, number];
    readonly bytecodemetadata_functionCount: (a: number) => number;
    readonly bytecodemetadata_getDependencies: (a: number) => [number, number];
    readonly bytecodemetadata_name: (a: number) => [number, number];
    readonly bytecodemetadata_structCount: (a: number) => number;
    readonly bytecodemetadata_toJSON: (a: number) => [number, number, number, number];
    readonly bytecodemetadata_version: (a: number) => number;
    readonly decompile_module: (a: number, b: number) => [number, number, number, number];
    readonly decompile_script: (a: number, b: number) => [number, number, number, number];
    readonly disassemble_module: (a: number, b: number) => [number, number, number, number];
    readonly disassemble_script: (a: number, b: number) => [number, number, number, number];
    readonly get_module_metadata: (a: number, b: number) => [number, number, number];
    readonly get_version_info: () => [number, number];
    readonly verify_module: (a: number, b: number) => [number, number, number];
    readonly verify_script: (a: number, b: number) => [number, number, number];
    readonly init_panic_hook: () => void;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __externref_table_dealloc: (a: number) => void;
    readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
