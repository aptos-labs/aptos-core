import {
  TypeTag,
  TypeTagBool,
  TypeTagU8,
  TypeTagU64,
  TypeTagU128,
  TypeTagAddress,
  AccountAddress,
  TypeTagVector,
  TypeTagStruct,
  StructTag,
  Identifier,
  TransactionArgument,
  TransactionArgumentBool,
  TransactionArgumentU64,
  TransactionArgumentU128,
  TransactionArgumentAddress,
  TransactionArgumentU8,
  TransactionArgumentU8Vector,
} from "./aptos_types";
import { Serializer } from "./bcs";

function assertType(val: any, types: string[] | string, message?: string) {
  if (!types?.includes(typeof val)) {
    throw new Error(
      message || `Invalid arg: ${val} type should be ${types instanceof Array ? types.join(" or ") : types}`,
    );
  }
}

/**
 * Parses a tag string
 * @param typeTagStr A string represented tag, e.g. bool
 * @returns
 */
export function parseTypeTag(typeTagStr: string): TypeTag {
  const typeTag = typeTagStr.trim();

  if (typeTag.startsWith("vector")) {
    // Strips off 'vector'
    let innerTagStr = typeTag.substring(6).trim();
    // Strips off '<' and '>'
    innerTagStr = innerTagStr.substring(1, innerTagStr.length - 1);
    return new TypeTagVector(parseTypeTag(innerTagStr));
  }

  if (typeTag.includes("::")) {
    if (!typeTag.includes("<")) {
      return new TypeTagStruct(StructTag.fromString(typeTag));
    }

    const [structStr, tempStrNotEndTrimmed] = typeTag.split("<", 2);

    // "0x1::Coin::CoinStore".match(/::/g) produces ['::', '::']
    if ((structStr.match(/::/g) || []).length !== 2) {
      throw new Error("Invalid struct tag string literal.");
    }

    const [address, module, name] = structStr.trim().split("::");

    const pos = tempStrNotEndTrimmed.lastIndexOf(">");
    if (pos === -1) {
      throw new Error("Invalid struct tag string literal.");
    }

    const tempStr = tempStrNotEndTrimmed.trim().substring(0, pos - 1);

    const tempStrParts = tempStr.split(",");

    const typeArgs = tempStrParts.map((temp) => parseTypeTag(temp));
    const structTag = new StructTag(
      AccountAddress.fromHex(address),
      new Identifier(module),
      new Identifier(name),
      typeArgs,
    );

    return new TypeTagStruct(structTag);
  }

  switch (typeTag) {
    case "bool":
      return new TypeTagBool();
    case "u8":
      return new TypeTagU8();
    case "u64":
      return new TypeTagU64();
    case "u128":
      return new TypeTagU128();
    case "address":
      return new TypeTagAddress();
    default:
      throw new Error("Unknown type tag.");
  }
}

export function serializeArg(argVal: any, argType: TypeTag, serializer: Serializer) {
  if (argType instanceof TypeTagBool) {
    assertType(argVal, "boolean");
    serializer.serializeBool(argVal);
    return;
  }
  if (argType instanceof TypeTagU8) {
    assertType(argVal, "number");
    serializer.serializeU8(argVal);
    return;
  }
  if (argType instanceof TypeTagU64) {
    assertType(argVal, ["number", "bigint"]);
    serializer.serializeU64(argVal);
    return;
  }
  if (argType instanceof TypeTagU128) {
    assertType(argVal, ["number", "bigint"]);
    serializer.serializeU128(argVal);
    return;
  }
  if (argType instanceof TypeTagAddress) {
    let addr: AccountAddress;
    if (typeof argVal === "string") {
      addr = AccountAddress.fromHex(argVal);
    } else if (argVal instanceof AccountAddress) {
      addr = argVal;
    } else {
      throw new Error("Invalid account address.");
    }
    addr.serialize(serializer);
    return;
  }
  if (argType instanceof TypeTagVector) {
    if (!(argVal instanceof Array)) {
      throw new Error("Invalid vector args.");
    }

    serializer.serializeU32AsUleb128(argVal.length);

    argVal.forEach((arg) => serializeArg(arg, argType.value, serializer));
    return;
  }
  throw new Error("Unsupported arg type.");
}

export function argToTransactionArgument(argVal: any, argType: TypeTag): TransactionArgument {
  if (argType instanceof TypeTagBool) {
    assertType(argVal, "boolean");
    return new TransactionArgumentBool(argVal);
  }
  if (argType instanceof TypeTagU8) {
    assertType(argVal, "number");
    return new TransactionArgumentU8(argVal);
  }
  if (argType instanceof TypeTagU64) {
    assertType(argVal, ["number", "bigint"]);
    return new TransactionArgumentU64(argVal);
  }
  if (argType instanceof TypeTagU128) {
    assertType(argVal, ["number", "bigint"]);
    return new TransactionArgumentU128(argVal);
  }
  if (argType instanceof TypeTagAddress) {
    let addr: AccountAddress;
    if (typeof argVal === "string") {
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
