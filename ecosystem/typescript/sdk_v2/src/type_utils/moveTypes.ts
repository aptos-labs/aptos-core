/**
 * Types from Move language
 */

export type MoveNonStructTypes = MovePrimitive | MoveVector | MoveObject | MoveOption;

export type MovePrimitive =
  | "bool"
  | "u8"
  | "u16"
  | "u32"
  | "u64"
  | "u128"
  | "u256"
  | "address"
  | "0x1::string::String";

export type MoveVector = `vector<${string}>`;

export type MoveObject = `0x1::object::Object<${string}>`;

export type MoveOption = `0x1::option::Option<${string}>`;
