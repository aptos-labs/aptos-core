/**
 * Convert Move type to TypeScript type,
 * for input arguments of view function or entry function.
 */

import { UnknownStruct } from "../common";
import { MoveNonStructTypes, MovePrimitive } from "../moveTypes";

/**
 * Convert an array of input arguments type.
 */
export type ConvertArgs<T extends readonly string[]> = T extends readonly [
  infer TArg extends string,
  ...infer TRest extends string[],
]
  ? [ConvertArgType<TArg>, ...ConvertArgs<TRest>]
  : [];

/**
 * Internal
 */
type ConvertArgType<TMoveType extends string> =
  TMoveType extends MoveNonStructTypes
    ? // it's a non-struct type
      ConvertNonStructArgType<TMoveType>
    : // it's a struct type
      UnknownStruct<TMoveType>;

type ConvertPrimitiveArgType<TMoveType extends MovePrimitive> =
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

type ConvertNonStructArgType<TMoveType extends MoveNonStructTypes> =
  TMoveType extends MovePrimitive
    ? ConvertPrimitiveArgType<TMoveType>
    : TMoveType extends `vector<${infer TInner}>`
    ? ConvertArgType<TInner>[]
    : TMoveType extends `0x1::object::Object<${string}>`
    ? `0x${string}`
    : TMoveType extends `0x1::option::Option<${infer TInner}>`
    ? ([ConvertArgType<TInner>] | [])
    : UnknownStruct<TMoveType>;
