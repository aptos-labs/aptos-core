/**
 * The types for the `Client` class.
 */

import { ABIRoot } from "../abi";
import {
  ExtractGenericArgsType,
  ViewFunctionName,
  ExtractArgsType,
} from "../extractor/functionExtractor";

/**
 * The input payload type of `createViewPayload`
 */
export type ViewRequestPayload<
  T extends ABIRoot,
> = {
  [TFuncName in ViewFunctionName<T>]: {
    function: `${T["address"]}::${T["name"]}::${TFuncName}`;
    arguments: ExtractArgsType<T, TFuncName>;
    type_arguments: ExtractGenericArgsType<T, TFuncName>;
  };
}[ViewFunctionName<T>];
