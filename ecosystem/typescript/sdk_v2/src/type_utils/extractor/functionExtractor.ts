/**
 * Extract function information from ABI JSON
 */

import { ABIRoot } from "../abi";
import { OmitSigner } from "../common";
import { ConvertArgs } from "../converter/argsConverter";
import { ConvertGenerics } from "../converter/genericConverter";
import { ConvertReturns } from "../converter/returnConverter";

/**
 * All view function names in the ABI.
 */
export type ViewFunctionName<T extends ABIRoot> = ViewFunction<T>["name"];

/**
 * All entry function names in the ABI.
 */
export type EntryFunctionName<T extends ABIRoot> = EntryFunction<T>["name"];

/**
 * Extract the return type of a function from ABI with function name.
 */
export type ExtractReturnType<
  T extends ABIRoot,
  TFuncName extends FunctionName<T>,
> = ConvertReturns<ExtractMoveReturnType<T, TFuncName>>;

/**
 * Extract the input arguments type of a function from ABI with function name.
 */
export type ExtractArgsType<
  T extends ABIRoot,
  TFuncName extends FunctionName<T>,
> = ConvertArgs<ExtractMoveArgsType<T, TFuncName>>;

/**
 * Extract the input arguments type of a function from ABI with function name, but omit the signer.
 */
export type ExtractArgsTypeOmitSigner<
  T extends ABIRoot,
  TFuncName extends FunctionName<T>,
> = ConvertArgs<OmitSigner<ExtractMoveArgsType<T, TFuncName>>>;

/**
 * Extract the input generic arguments type of a function from ABI with function name.
 */
export type ExtractGenericArgsType<
  T extends ABIRoot,
  TFuncName extends FunctionName<T>,
> = ConvertGenerics<ExtractMoveGenericParamsType<T, TFuncName>>;

/**
 * Internal
 */
type Functions<T extends ABIRoot> = T["exposed_functions"];
type Function<T extends ABIRoot> = Functions<T>[number];
type FunctionName<T extends ABIRoot> = Function<T>["name"];
type ViewFunction<T extends ABIRoot> = Extract<
  Functions<T>[number],
  { is_view: true }
>;
type EntryFunction<T extends ABIRoot> = Extract<
  Functions<T>[number],
  { is_entry: true }
>;

type ExtractFunction<
  T extends ABIRoot,
  TFuncName extends FunctionName<T>,
> = Extract<Function<T>, { name: TFuncName }>;

type ExtractMoveReturnType<
  T extends ABIRoot,
  TFuncName extends FunctionName<T>,
> = ExtractFunction<T, TFuncName>["return"];

type ExtractMoveArgsType<
  T extends ABIRoot,
  TFuncName extends FunctionName<T>,
> = ExtractFunction<T, TFuncName>["params"];

type ExtractMoveGenericParamsType<
  T extends ABIRoot,
  TFuncName extends FunctionName<T>,
> = ExtractFunction<T, TFuncName>["generic_type_params"];
