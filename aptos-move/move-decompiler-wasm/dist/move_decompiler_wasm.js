/* @ts-self-types="./move_decompiler_wasm.d.ts" */

/**
 * Metadata extracted from Move bytecode
 */
export class BytecodeMetadata {
    static __wrap(ptr) {
        const obj = Object.create(BytecodeMetadata.prototype);
        obj.__wbg_ptr = ptr;
        BytecodeMetadataFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        BytecodeMetadataFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_bytecodemetadata_free(ptr, 0);
    }
    /**
     * Get module address
     * @returns {string | undefined}
     */
    get address() {
        const ret = wasm.bytecodemetadata_address(this.__wbg_ptr);
        let v1;
        if (ret[0] !== 0) {
            v1 = getStringFromWasm0(ret[0], ret[1]).slice();
            wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
        }
        return v1;
    }
    /**
     * Get function count
     * @returns {number}
     */
    get functionCount() {
        const ret = wasm.bytecodemetadata_functionCount(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * Get dependencies as JSON array string
     * @returns {string}
     */
    getDependencies() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.bytecodemetadata_getDependencies(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Get module name
     * @returns {string}
     */
    get name() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.bytecodemetadata_name(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Get struct count
     * @returns {number}
     */
    get structCount() {
        const ret = wasm.bytecodemetadata_structCount(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * Convert to JSON string
     * @returns {string}
     */
    toJSON() {
        let deferred2_0;
        let deferred2_1;
        try {
            const ret = wasm.bytecodemetadata_toJSON(this.__wbg_ptr);
            var ptr1 = ret[0];
            var len1 = ret[1];
            if (ret[3]) {
                ptr1 = 0; len1 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred2_0 = ptr1;
            deferred2_1 = len1;
            return getStringFromWasm0(ptr1, len1);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * Get bytecode version
     * @returns {number}
     */
    get version() {
        const ret = wasm.bytecodemetadata_version(this.__wbg_ptr);
        return ret >>> 0;
    }
}
if (Symbol.dispose) BytecodeMetadata.prototype[Symbol.dispose] = BytecodeMetadata.prototype.free;

/**
 * Compile Move module source code to bytecode (bytecode v10).
 *
 * # Arguments
 * * `source` - Move source text defining exactly one module
 * * `named_addresses_json` - JSON object mapping named addresses to hex
 *   addresses, e.g. `{"my_addr":"0x42"}`. May be empty (`""`) if the source
 *   only uses numeric addresses.
 *
 * # Returns
 * Serialized module bytecode (a `.mv` byte array) at bytecode version 10.
 *
 * # Errors
 * Returns an error (with compiler diagnostics) if compilation fails or the
 * source does not define a module.
 * @param {string} source
 * @param {string} named_addresses_json
 * @returns {Uint8Array}
 */
export function compile_module(source, named_addresses_json) {
    const ptr0 = passStringToWasm0(source, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    const ptr1 = passStringToWasm0(named_addresses_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len1 = WASM_VECTOR_LEN;
    const ret = wasm.compile_module(ptr0, len0, ptr1, len1);
    if (ret[3]) {
        throw takeFromExternrefTable0(ret[2]);
    }
    var v3 = getArrayU8FromWasm0(ret[0], ret[1]).slice();
    wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
    return v3;
}

/**
 * Compile Move script source code to bytecode (bytecode v10).
 *
 * # Arguments
 * * `source` - Move source text defining a script (a `script { ... }` block)
 * * `named_addresses_json` - JSON object mapping named addresses to hex
 *   addresses. May be empty (`""`).
 *
 * # Returns
 * Serialized script bytecode at bytecode version 10.
 * @param {string} source
 * @param {string} named_addresses_json
 * @returns {Uint8Array}
 */
export function compile_script(source, named_addresses_json) {
    const ptr0 = passStringToWasm0(source, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    const ptr1 = passStringToWasm0(named_addresses_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len1 = WASM_VECTOR_LEN;
    const ret = wasm.compile_script(ptr0, len0, ptr1, len1);
    if (ret[3]) {
        throw takeFromExternrefTable0(ret[2]);
    }
    var v3 = getArrayU8FromWasm0(ret[0], ret[1]).slice();
    wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
    return v3;
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
 * @param {Uint8Array} bytecode
 * @returns {string}
 */
export function decompile_module(bytecode) {
    let deferred3_0;
    let deferred3_1;
    try {
        const ptr0 = passArray8ToWasm0(bytecode, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.decompile_module(ptr0, len0);
        var ptr2 = ret[0];
        var len2 = ret[1];
        if (ret[3]) {
            ptr2 = 0; len2 = 0;
            throw takeFromExternrefTable0(ret[2]);
        }
        deferred3_0 = ptr2;
        deferred3_1 = len2;
        return getStringFromWasm0(ptr2, len2);
    } finally {
        wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
    }
}

/**
 * Decompile Move script bytecode to Move source code
 *
 * # Arguments
 * * `bytecode` - Raw bytes from a compiled Move script
 *
 * # Returns
 * Decompiled Move script source code
 * @param {Uint8Array} bytecode
 * @returns {string}
 */
export function decompile_script(bytecode) {
    let deferred3_0;
    let deferred3_1;
    try {
        const ptr0 = passArray8ToWasm0(bytecode, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.decompile_script(ptr0, len0);
        var ptr2 = ret[0];
        var len2 = ret[1];
        if (ret[3]) {
            ptr2 = 0; len2 = 0;
            throw takeFromExternrefTable0(ret[2]);
        }
        deferred3_0 = ptr2;
        deferred3_1 = len2;
        return getStringFromWasm0(ptr2, len2);
    } finally {
        wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
    }
}

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
 * @param {Uint8Array} bytecode
 * @returns {string}
 */
export function disassemble_module(bytecode) {
    let deferred3_0;
    let deferred3_1;
    try {
        const ptr0 = passArray8ToWasm0(bytecode, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.disassemble_module(ptr0, len0);
        var ptr2 = ret[0];
        var len2 = ret[1];
        if (ret[3]) {
            ptr2 = 0; len2 = 0;
            throw takeFromExternrefTable0(ret[2]);
        }
        deferred3_0 = ptr2;
        deferred3_1 = len2;
        return getStringFromWasm0(ptr2, len2);
    } finally {
        wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
    }
}

/**
 * Disassemble Move script bytecode
 *
 * # Arguments
 * * `bytecode` - Raw bytes from a compiled Move script
 *
 * # Returns
 * Disassembled script bytecode
 * @param {Uint8Array} bytecode
 * @returns {string}
 */
export function disassemble_script(bytecode) {
    let deferred3_0;
    let deferred3_1;
    try {
        const ptr0 = passArray8ToWasm0(bytecode, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.disassemble_script(ptr0, len0);
        var ptr2 = ret[0];
        var len2 = ret[1];
        if (ret[3]) {
            ptr2 = 0; len2 = 0;
            throw takeFromExternrefTable0(ret[2]);
        }
        deferred3_0 = ptr2;
        deferred3_1 = len2;
        return getStringFromWasm0(ptr2, len2);
    } finally {
        wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
    }
}

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
 * @param {Uint8Array} bytecode
 * @returns {BytecodeMetadata}
 */
export function get_module_metadata(bytecode) {
    const ptr0 = passArray8ToWasm0(bytecode, wasm.__wbindgen_malloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.get_module_metadata(ptr0, len0);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return BytecodeMetadata.__wrap(ret[0]);
}

/**
 * Get library version information
 *
 * Returns version and build information for the WASM library.
 *
 * # Returns
 * JSON string with version details
 * @returns {string}
 */
export function get_version_info() {
    let deferred1_0;
    let deferred1_1;
    try {
        const ret = wasm.get_version_info();
        deferred1_0 = ret[0];
        deferred1_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
    }
}

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
export function init_panic_hook() {
    wasm.init_panic_hook();
}

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
 * @param {Uint8Array} bytecode
 * @returns {boolean}
 */
export function verify_module(bytecode) {
    const ptr0 = passArray8ToWasm0(bytecode, wasm.__wbindgen_malloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.verify_module(ptr0, len0);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return ret[0] !== 0;
}

/**
 * Verify Move script bytecode integrity
 * @param {Uint8Array} bytecode
 * @returns {boolean}
 */
export function verify_script(bytecode) {
    const ptr0 = passArray8ToWasm0(bytecode, wasm.__wbindgen_malloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.verify_script(ptr0, len0);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return ret[0] !== 0;
}
function __wbg_get_imports() {
    const import0 = {
        __proto__: null,
        __wbg___wbindgen_throw_1506f2235d1bdba0: function(arg0, arg1) {
            throw new Error(getStringFromWasm0(arg0, arg1));
        },
        __wbg_error_a6fa202b58aa1cd3: function(arg0, arg1) {
            let deferred0_0;
            let deferred0_1;
            try {
                deferred0_0 = arg0;
                deferred0_1 = arg1;
                console.error(getStringFromWasm0(arg0, arg1));
            } finally {
                wasm.__wbindgen_free(deferred0_0, deferred0_1, 1);
            }
        },
        __wbg_new_227d7c05414eb861: function() {
            const ret = new Error();
            return ret;
        },
        __wbg_stack_3b0d974bbf31e44f: function(arg0, arg1) {
            const ret = arg1.stack;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbindgen_cast_0000000000000001: function(arg0, arg1) {
            // Cast intrinsic for `Ref(String) -> Externref`.
            const ret = getStringFromWasm0(arg0, arg1);
            return ret;
        },
        __wbindgen_init_externref_table: function() {
            const table = wasm.__wbindgen_externrefs;
            const offset = table.grow(4);
            table.set(0, undefined);
            table.set(offset + 0, undefined);
            table.set(offset + 1, null);
            table.set(offset + 2, true);
            table.set(offset + 3, false);
        },
    };
    return {
        __proto__: null,
        "./move_decompiler_wasm_bg.js": import0,
    };
}

const BytecodeMetadataFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_bytecodemetadata_free(ptr, 1));

function getArrayU8FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getUint8ArrayMemory0().subarray(ptr / 1, ptr / 1 + len);
}

let cachedDataViewMemory0 = null;
function getDataViewMemory0() {
    if (cachedDataViewMemory0 === null || cachedDataViewMemory0.buffer.detached === true || (cachedDataViewMemory0.buffer.detached === undefined && cachedDataViewMemory0.buffer !== wasm.memory.buffer)) {
        cachedDataViewMemory0 = new DataView(wasm.memory.buffer);
    }
    return cachedDataViewMemory0;
}

function getStringFromWasm0(ptr, len) {
    return decodeText(ptr >>> 0, len);
}

let cachedUint8ArrayMemory0 = null;
function getUint8ArrayMemory0() {
    if (cachedUint8ArrayMemory0 === null || cachedUint8ArrayMemory0.byteLength === 0) {
        cachedUint8ArrayMemory0 = new Uint8Array(wasm.memory.buffer);
    }
    return cachedUint8ArrayMemory0;
}

function passArray8ToWasm0(arg, malloc) {
    const ptr = malloc(arg.length * 1, 1) >>> 0;
    getUint8ArrayMemory0().set(arg, ptr / 1);
    WASM_VECTOR_LEN = arg.length;
    return ptr;
}

function passStringToWasm0(arg, malloc, realloc) {
    if (realloc === undefined) {
        const buf = cachedTextEncoder.encode(arg);
        const ptr = malloc(buf.length, 1) >>> 0;
        getUint8ArrayMemory0().subarray(ptr, ptr + buf.length).set(buf);
        WASM_VECTOR_LEN = buf.length;
        return ptr;
    }

    let len = arg.length;
    let ptr = malloc(len, 1) >>> 0;

    const mem = getUint8ArrayMemory0();

    let offset = 0;

    for (; offset < len; offset++) {
        const code = arg.charCodeAt(offset);
        if (code > 0x7F) break;
        mem[ptr + offset] = code;
    }
    if (offset !== len) {
        if (offset !== 0) {
            arg = arg.slice(offset);
        }
        ptr = realloc(ptr, len, len = offset + arg.length * 3, 1) >>> 0;
        const view = getUint8ArrayMemory0().subarray(ptr + offset, ptr + len);
        const ret = cachedTextEncoder.encodeInto(arg, view);

        offset += ret.written;
        ptr = realloc(ptr, len, offset, 1) >>> 0;
    }

    WASM_VECTOR_LEN = offset;
    return ptr;
}

function takeFromExternrefTable0(idx) {
    const value = wasm.__wbindgen_externrefs.get(idx);
    wasm.__externref_table_dealloc(idx);
    return value;
}

let cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true, fatal: true });
cachedTextDecoder.decode();
const MAX_SAFARI_DECODE_BYTES = 2146435072;
let numBytesDecoded = 0;
function decodeText(ptr, len) {
    numBytesDecoded += len;
    if (numBytesDecoded >= MAX_SAFARI_DECODE_BYTES) {
        cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true, fatal: true });
        cachedTextDecoder.decode();
        numBytesDecoded = len;
    }
    return cachedTextDecoder.decode(getUint8ArrayMemory0().subarray(ptr, ptr + len));
}

const cachedTextEncoder = new TextEncoder();

if (!('encodeInto' in cachedTextEncoder)) {
    cachedTextEncoder.encodeInto = function (arg, view) {
        const buf = cachedTextEncoder.encode(arg);
        view.set(buf);
        return {
            read: arg.length,
            written: buf.length
        };
    };
}

let WASM_VECTOR_LEN = 0;

let wasmModule, wasmInstance, wasm;
function __wbg_finalize_init(instance, module) {
    wasmInstance = instance;
    wasm = instance.exports;
    wasmModule = module;
    cachedDataViewMemory0 = null;
    cachedUint8ArrayMemory0 = null;
    wasm.__wbindgen_start();
    return wasm;
}

async function __wbg_load(module, imports) {
    if (typeof Response === 'function' && module instanceof Response) {
        if (typeof WebAssembly.instantiateStreaming === 'function') {
            try {
                return await WebAssembly.instantiateStreaming(module, imports);
            } catch (e) {
                const validResponse = module.ok && expectedResponseType(module.type);

                if (validResponse && module.headers.get('Content-Type') !== 'application/wasm') {
                    console.warn("`WebAssembly.instantiateStreaming` failed because your server does not serve Wasm with `application/wasm` MIME type. Falling back to `WebAssembly.instantiate` which is slower. Original error:\n", e);

                } else { throw e; }
            }
        }

        const bytes = await module.arrayBuffer();
        return await WebAssembly.instantiate(bytes, imports);
    } else {
        const instance = await WebAssembly.instantiate(module, imports);

        if (instance instanceof WebAssembly.Instance) {
            return { instance, module };
        } else {
            return instance;
        }
    }

    function expectedResponseType(type) {
        switch (type) {
            case 'basic': case 'cors': case 'default': return true;
        }
        return false;
    }
}

function initSync(module) {
    if (wasm !== undefined) return wasm;


    if (module !== undefined) {
        if (Object.getPrototypeOf(module) === Object.prototype) {
            ({module} = module)
        } else {
            console.warn('using deprecated parameters for `initSync()`; pass a single object instead')
        }
    }

    const imports = __wbg_get_imports();
    if (!(module instanceof WebAssembly.Module)) {
        module = new WebAssembly.Module(module);
    }
    const instance = new WebAssembly.Instance(module, imports);
    return __wbg_finalize_init(instance, module);
}

async function __wbg_init(module_or_path) {
    if (wasm !== undefined) return wasm;


    if (module_or_path !== undefined) {
        if (Object.getPrototypeOf(module_or_path) === Object.prototype) {
            ({module_or_path} = module_or_path)
        } else {
            console.warn('using deprecated parameters for the initialization function; pass a single object instead')
        }
    }

    if (module_or_path === undefined) {
        module_or_path = new URL('move_decompiler_wasm_bg.wasm', import.meta.url);
    }
    const imports = __wbg_get_imports();

    if (typeof module_or_path === 'string' || (typeof Request === 'function' && module_or_path instanceof Request) || (typeof URL === 'function' && module_or_path instanceof URL)) {
        module_or_path = fetch(module_or_path);
    }

    const { instance, module } = await __wbg_load(await module_or_path, imports);

    return __wbg_finalize_init(instance, module);
}

export { initSync, __wbg_init as default };
