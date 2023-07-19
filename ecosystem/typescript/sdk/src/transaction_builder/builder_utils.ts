// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

import { HexString } from "../utils";
import {
  TypeTag,
  TypeTagBool,
  TypeTagU8,
  TypeTagU16,
  TypeTagU32,
  TypeTagU64,
  TypeTagU128,
  TypeTagU256,
  TypeTagAddress,
  AccountAddress,
  TypeTagVector,
  TypeTagStruct,
  TransactionArgument,
  TransactionArgumentBool,
  TransactionArgumentU16,
  TransactionArgumentU32,
  TransactionArgumentU64,
  TransactionArgumentU128,
  TransactionArgumentU256,
  TransactionArgumentAddress,
  TransactionArgumentU8,
  TransactionArgumentU8Vector,
} from "../aptos_types";
import { Serializer } from "../bcs";

function assertType(val: any, types: string[] | string, message?: string) {
  if (!types?.includes(typeof val)) {
    throw new Error(
      message || `Invalid arg: ${val} type should be ${types instanceof Array ? types.join(" or ") : types}`,
    );
  }
}

export function ensureBoolean(val: boolean | string): boolean {
  assertType(val, ["boolean", "string"]);
  if (typeof val === "boolean") {
    return val;
  }

  if (val === "true") {
    return true;
  }
  if (val === "false") {
    return false;
  }

  throw new Error("Invalid boolean string.");
}

export function ensureNumber(val: number | string): number {
  assertType(val, ["number", "string"]);
  if (typeof val === "number") {
    return val;
  }

  const res = Number.parseInt(val, 10);
  if (Number.isNaN(res)) {
    throw new Error("Invalid number string.");
  }

  return res;
}

export function ensureBigInt(val: number | bigint | string): bigint {
  assertType(val, ["number", "bigint", "string"]);
  return BigInt(val);
}

export function serializeArg(argVal: any, argType: TypeTag, serializer: Serializer) {
  serializeArgInner(argVal, argType, serializer, 0);
}

function serializeArgInner(argVal: any, argType: TypeTag, serializer: Serializer, depth: number) {
  if (argType instanceof TypeTagBool) {
    serializer.serializeBool(ensureBoolean(argVal));
  } else if (argType instanceof TypeTagU8) {
    serializer.serializeU8(ensureNumber(argVal));
  } else if (argType instanceof TypeTagU16) {
    serializer.serializeU16(ensureNumber(argVal));
  } else if (argType instanceof TypeTagU32) {
    serializer.serializeU32(ensureNumber(argVal));
  } else if (argType instanceof TypeTagU64) {
    serializer.serializeU64(ensureBigInt(argVal));
  } else if (argType instanceof TypeTagU128) {
    serializer.serializeU128(ensureBigInt(argVal));
  } else if (argType instanceof TypeTagU256) {
    serializer.serializeU256(ensureBigInt(argVal));
  } else if (argType instanceof TypeTagAddress) {
    serializeAddress(argVal, serializer);
  } else if (argType instanceof TypeTagVector) {
    serializeVector(argVal, argType, serializer, depth);
  } else if (argType instanceof TypeTagStruct) {
    serializeStruct(argVal, argType, serializer, depth);
  } else {
    throw new Error("Unsupported arg type.");
  }
}

function serializeAddress(argVal: any, serializer: Serializer) {
  let addr: AccountAddress;
  if (typeof argVal === "string" || argVal instanceof HexString) {
    addr = AccountAddress.fromHex(argVal);
  } else if (argVal instanceof AccountAddress) {
    addr = argVal;
  } else {
    throw new Error("Invalid account address.");
  }
  addr.serialize(serializer);
}

function serializeVector(argVal: any, argType: TypeTagVector, serializer: Serializer, depth: number) {
  // We are serializing a vector<u8>
  if (argType.value instanceof TypeTagU8) {
    if (argVal instanceof Uint8Array) {
      serializer.serializeBytes(argVal);
      return;
    }
    if (argVal instanceof HexString) {
      serializer.serializeBytes(argVal.toUint8Array());
      return;
    }
    if (typeof argVal === "string") {
      serializer.serializeStr(argVal);
      return;
    }
    // If it isn't any of those types, then it must just be an actual array of numbers
  }

  if (!Array.isArray(argVal)) {
    throw new Error("Invalid vector args.");
  }

  serializer.serializeU32AsUleb128(argVal.length);

  argVal.forEach((arg) => serializeArgInner(arg, argType.value, serializer, depth + 1));
}

function serializeStruct(argVal: any, argType: TypeTag, serializer: Serializer, depth: number) {
  const { address, module_name: moduleName, name, type_args: typeArgs } = (argType as TypeTagStruct).value;
  const structType = `${HexString.fromUint8Array(address.address).toShortString()}::${moduleName.value}::${name.value}`;
  if (structType === "0x1::string::String") {
    assertType(argVal, ["string"]);
    serializer.serializeStr(argVal);
  } else if (structType === "0x1::object::Object") {
    serializeAddress(argVal, serializer);
  } else if (structType === "0x1::option::Option") {
    if (typeArgs.length !== 1) {
      throw new Error(`Option has the wrong number of type arguments ${typeArgs.length}`);
    }
    serializeOption(argVal, typeArgs[0], serializer, depth);
  } else {
    throw new Error("Unsupported struct type in function argument");
  }
}

function serializeOption(argVal: any, argType: TypeTag, serializer: Serializer, depth: number) {
  // For option, we determine if it's empty or not empty first
  // empty option is nothing, we specifically check for undefined to prevent fuzzy matching
  if (argVal === undefined || argVal === null) {
    serializer.serializeU32AsUleb128(0);
  } else {
    // Something means we need an array of 1
    serializer.serializeU32AsUleb128(1);

    // Serialize the inner type arg, ensuring that depth is tracked
    serializeArgInner(argVal, argType, serializer, depth + 1);
  }
}

export function argToTransactionArgument(argVal: any, argType: TypeTag): TransactionArgument {
  if (argType instanceof TypeTagBool) {
    return new TransactionArgumentBool(ensureBoolean(argVal));
  }
  if (argType instanceof TypeTagU8) {
    return new TransactionArgumentU8(ensureNumber(argVal));
  }
  if (argType instanceof TypeTagU16) {
    return new TransactionArgumentU16(ensureNumber(argVal));
  }
  if (argType instanceof TypeTagU32) {
    return new TransactionArgumentU32(ensureNumber(argVal));
  }
  if (argType instanceof TypeTagU64) {
    return new TransactionArgumentU64(ensureBigInt(argVal));
  }
  if (argType instanceof TypeTagU128) {
    return new TransactionArgumentU128(ensureBigInt(argVal));
  }
  if (argType instanceof TypeTagU256) {
    return new TransactionArgumentU256(ensureBigInt(argVal));
  }
  if (argType instanceof TypeTagAddress) {
    let addr: AccountAddress;
    if (typeof argVal === "string" || argVal instanceof HexString) {
      addr = AccountAddress.fromHex(argVal);
    } else if (argVal instanceof AccountAddress) {
      addr = argVal;
    } else {
      throw new Error("Invalid account address.");
    }
    return new TransactionArgumentAddress(addr);
  }
  if (argType instanceof TypeTagVector && argType.value instanceof TypeTagU8) {
    if (!(argVal instanceof Uint8Array)) {
      throw new Error(`${argVal} should be an instance of Uint8Array`);
    }
    return new TransactionArgumentU8Vector(argVal);
  }

  throw new Error("Unknown type for TransactionArgument.");
}
