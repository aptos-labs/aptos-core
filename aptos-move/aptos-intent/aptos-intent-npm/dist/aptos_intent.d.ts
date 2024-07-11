/* tslint:disable */
/* eslint-disable */

import * as exp from "constants";

/**
* Arguments for each function.
*/
export enum BatchArgumentType {
  Raw = 0,
  Signer = 1,
  PreviousResult = 2,
}
/**
*/
export enum ArgumentOperation {
  Move = 0,
  Copy = 1,
  Borrow = 2,
  BorrowMut = 3,
}
/**
* Arguments for each function. Wasm bindgen only support C-style enum so use option to work around.
*/
export class BatchArgument {
  free(): void;
/**
* @param {Uint8Array} bytes
* @returns {BatchArgument}
*/
  static new_bytes(bytes: Uint8Array): BatchArgument;
/**
* @param {number} signer_idx
* @returns {BatchArgument}
*/
  static new_signer(signer_idx: number): BatchArgument;
/**
* @returns {BatchArgument}
*/
  borrow(): BatchArgument;
/**
* @returns {BatchArgument}
*/
  borrow_mut(): BatchArgument;
/**
* @returns {BatchArgument}
*/
  copy(): BatchArgument;
}
/**
* Call a Move entry function.
*/
export class BatchedFunctionCall {
  free(): void;
}
/**
*/
export class BatchedFunctionCallBuilder {
  free(): void;
/**
* @returns {BatchedFunctionCallBuilder}
*/
  static single_signer(): BatchedFunctionCallBuilder;
/**
* @param {number} signer_count
* @returns {BatchedFunctionCallBuilder}
*/
  static multi_signer(signer_count: number): BatchedFunctionCallBuilder;
/**
* @param {string} module
* @param {string} _function
* @param {(string)[]} ty_args
* @param {(BatchArgument)[]} args
* @returns {(BatchArgument)[]}
*/
  add_batched_call(module: string, _function: string, ty_args: (string)[], args: (BatchArgument)[]): (BatchArgument)[];
/**
* @returns {Uint8Array}
*/
  generate_batched_calls(): Uint8Array;
/**
* @param {string} api_url
* @param {string} module_name
* @returns {Promise<void>}
*/
  load_module(api_url: string, module_name: string): Promise<void>;
}
/**
*/
export class PreviousResult {
  free(): void;
}

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
  readonly memory: WebAssembly.Memory;
  readonly __wbg_previousresult_free: (a: number) => void;
  readonly __wbg_batchargument_free: (a: number) => void;
  readonly __wbg_batchedfunctioncall_free: (a: number) => void;
  readonly __wbg_batchedfunctioncallbuilder_free: (a: number) => void;
  readonly batchedfunctioncallbuilder_single_signer: () => number;
  readonly batchedfunctioncallbuilder_multi_signer: (a: number) => number;
  readonly batchedfunctioncallbuilder_add_batched_call: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number, j: number) => void;
  readonly batchedfunctioncallbuilder_generate_batched_calls: (a: number, b: number) => void;
  readonly batchedfunctioncallbuilder_load_module: (a: number, b: number, c: number, d: number, e: number) => number;
  readonly batchargument_new_bytes: (a: number, b: number) => number;
  readonly batchargument_new_signer: (a: number) => number;
  readonly batchargument_borrow: (a: number, b: number) => void;
  readonly batchargument_borrow_mut: (a: number, b: number) => void;
  readonly batchargument_copy: (a: number, b: number) => void;
  readonly __wbindgen_malloc: (a: number, b: number) => number;
  readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
  readonly __wbindgen_export_2: WebAssembly.Table;
  readonly _dyn_core__ops__function__FnMut__A____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__h09acdaa8b02601d5: (a: number, b: number, c: number) => void;
  readonly __wbindgen_add_to_stack_pointer: (a: number) => number;
  readonly __wbindgen_free: (a: number, b: number, c: number) => void;
  readonly __wbindgen_exn_store: (a: number) => void;
  readonly wasm_bindgen__convert__closures__invoke2_mut__h4c0838795c3445c5: (a: number, b: number, c: number, d: number) => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;
/**
* Instantiates the given `module`, which can either be bytes or
* a precompiled `WebAssembly.Module`.
*
* @param {SyncInitInput} module
*
* @returns {InitOutput}
*/
export function initSync(module: SyncInitInput): InitOutput;

/**
* If `module_or_path` is {RequestInfo} or {URL}, makes a request and
* for everything else, calls `WebAssembly.instantiate` directly.
*
* @param {InitInput | Promise<InitInput>} module_or_path
*
* @returns {Promise<InitOutput>}
*/
export function __wbg_init (module_or_path?: InitInput | Promise<InitInput>): Promise<InitOutput>;

export function get_wasm(): any