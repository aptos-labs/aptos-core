/**
 * Convert Move type to TypeScript type,
 * for return value of view functions.
 */

import { UnknownStruct } from "../common";
import { MoveNonStructTypes, MovePrimitive } from "../moveTypes";

/**
 * Convert an array of return types.
 */
export type ConvertReturns<T extends readonly string[]> = T extends readonly [
  infer TArg extends string,
  ...infer TRest extends string[],
]
  ? [ConvertReturnType<TArg>, ...ConvertReturns<TRest>]
  : [];

/**
 * Internal
 */
type ConvertReturnType<TMoveType extends string> =
  TMoveType extends MoveNonStructTypes
    ? // it's a non-struct type
      ConvertNonStructReturnType<TMoveType>
    : // it's a struct type
      UnknownStruct<TMoveType>;

type ConvertPrimitiveReturnType<TMoveType extends MovePrimitive> =
  TMoveType extends "bool"
    ? boolean
    : TMoveType extends "u8"
    ? number
    : TMoveType extends "u16"
    ? number
    : TMoveType extends "u32"
    ? number
    : TMoveType extends "u64"
    ? string
    : TMoveType extends "u128"
    ? string
    : TMoveType extends "u256"
    ? string
    : TMoveType extends "address"
    ? `0x${string}`
    : TMoveType extends "0x1::string::String"
    ? string
    : never;

type ConvertNonStructReturnType<TMoveType extends MoveNonStructTypes> =
  TMoveType extends MovePrimitive
    ? ConvertPrimitiveReturnType<TMoveType>
    : TMoveType extends `vector<${infer TInner}>`
    ? ConvertReturnType<TInner>[]
    : UnknownStruct<TMoveType>;
