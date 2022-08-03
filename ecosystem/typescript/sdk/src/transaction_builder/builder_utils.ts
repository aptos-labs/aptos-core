import { HexString } from "../hex_string";
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

function bail(message: string) {
  throw new Error(message);
}

function isWhiteSpace(c: string): boolean {
  if (c.match(/\s/)) {
    return true;
  }
  return false;
}

function isValidAlphabetic(c: string): boolean {
  if (c.match(/[_A-Za-z0-9]/g)) {
    return true;
  }
  return false;
}

type TokenType = string;
type TokenValue = string;
type Token = [TokenType, TokenValue];

// Returns Token and Token byte size
function nextToken(tagStr: string, pos: number): [Token, number] {
  const c = tagStr[pos];
  if (c === ":") {
    if (tagStr.slice(pos, pos + 2) === "::") {
      return [["COLON", "::"], 2];
    }
    bail("Unrecognized token.");
  } else if (c === "<") {
    return [["LT", "<"], 1];
  } else if (c === ">") {
    return [["GT", ">"], 1];
  } else if (c === ",") {
    return [["COMMA", ","], 1];
  } else if (isWhiteSpace(c)) {
    let res = "";
    for (let i = pos; i < tagStr.length; i += 1) {
      const char = tagStr[i];
      if (isWhiteSpace(char)) {
        res = `${res}${char}`;
      } else {
        break;
      }
    }
    return [["SPACE", res], res.length];
  } else if (isValidAlphabetic(c)) {
    let res = "";
    for (let i = pos; i < tagStr.length; i += 1) {
      const char = tagStr[i];
      if (isValidAlphabetic(char)) {
        res = `${res}${char}`;
      } else {
        break;
      }
    }
    return [["IDENT", res], res.length];
  }
  throw new Error("Unrecognized token.");
}

function tokenize(tagStr: string): Token[] {
  let pos = 0;
  const tokens = [];
  while (pos < tagStr.length) {
    const [token, size] = nextToken(tagStr, pos);
    if (token[0] !== "SPACE") {
      tokens.push(token);
    }
    pos += size;
  }
  return tokens;
}

/**
 * Parser to parse a type tag string
 */
export class TypeTagParser {
  private readonly tokens: Token[];

  constructor(tagStr: string) {
    this.tokens = tokenize(tagStr);
  }

  private consume(targetToken: string) {
    const token = this.tokens.shift();
    if (!token || token[1] !== targetToken) {
      bail("Invalid type tag.");
    }
  }

  private parseCommaList(endToken: TokenValue, allowTraillingComma: boolean): TypeTag[] {
    const res: TypeTag[] = [];
    if (this.tokens.length <= 0) {
      bail("Invalid type tag.");
    }

    while (this.tokens[0][1] !== endToken) {
      res.push(this.parseTypeTag());

      if (this.tokens.length > 0 && this.tokens[0][1] === endToken) {
        break;
      }

      this.consume(",");
      if (this.tokens.length > 0 && this.tokens[0][1] === endToken && allowTraillingComma) {
        break;
      }

      if (this.tokens.length <= 0) {
        bail("Invalid type tag.");
      }
    }
    return res;
  }

  parseTypeTag(): TypeTag {
    if (this.tokens.length === 0) {
      bail("Invalid type tag.");
    }

    // Pop left most element out
    const [tokenTy, tokenVal] = this.tokens.shift();

    if (tokenVal === "u8") {
      return new TypeTagU8();
    }
    if (tokenVal === "u64") {
      return new TypeTagU64();
    }
    if (tokenVal === "u128") {
      return new TypeTagU128();
    }
    if (tokenVal === "bool") {
      return new TypeTagBool();
    }
    if (tokenVal === "address") {
      return new TypeTagAddress();
    }
    if (tokenVal === "vector") {
      this.consume("<");
      const res = this.parseTypeTag();
      this.consume(">");
      return new TypeTagVector(res);
    }
    if (tokenTy === "IDENT" && (tokenVal.startsWith("0x") || tokenVal.startsWith("0X"))) {
      const address = tokenVal;
      this.consume("::");
      const [moduleTokenTy, module] = this.tokens.shift();
      if (moduleTokenTy !== "IDENT") {
        bail("Invalid type tag.");
      }
      this.consume("::");
      const [nameTokenTy, name] = this.tokens.shift();
      if (nameTokenTy !== "IDENT") {
        bail("Invalid type tag.");
      }

      let tyTags: TypeTag[] = [];
      // Check if the struct has ty args
      if (this.tokens.length > 0 && this.tokens[0][1] === "<") {
        this.consume("<");
        tyTags = this.parseCommaList(">", true);
        this.consume(">");
      }

      const structTag = new StructTag(
        AccountAddress.fromHex(address),
        new Identifier(module),
        new Identifier(name),
        tyTags,
      );
      return new TypeTagStruct(structTag);
    }

    throw new Error("Invalid type tag.");
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
    if (typeof argVal === "string" || argVal instanceof HexString) {
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
    // We are serializing a vector<u8>
    if (argType.value instanceof TypeTagU8) {
      if (argVal instanceof Uint8Array) {
        serializer.serializeBytes(argVal);
        return;
      }

      if (typeof argVal === "string") {
        serializer.serializeStr(argVal);
        return;
      }
    }

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
