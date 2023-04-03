// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

/* eslint-disable @typescript-eslint/no-unused-vars */
/* eslint-disable class-methods-use-this */
/* eslint-disable max-classes-per-file */
import { AccountAddress } from "./account_address";
import { Deserializer, Seq, Serializer, deserializeVector, serializeVector } from "../bcs";
import { Identifier } from "./identifier";

export abstract class TypeTag {
  abstract serialize(serializer: Serializer): void;

  static deserialize(deserializer: Deserializer): TypeTag {
    const index = deserializer.deserializeUleb128AsU32();
    switch (index) {
      case 0:
        return TypeTagBool.load(deserializer);
      case 1:
        return TypeTagU8.load(deserializer);
      case 2:
        return TypeTagU64.load(deserializer);
      case 3:
        return TypeTagU128.load(deserializer);
      case 4:
        return TypeTagAddress.load(deserializer);
      case 5:
        return TypeTagSigner.load(deserializer);
      case 6:
        return TypeTagVector.load(deserializer);
      case 7:
        return TypeTagStruct.load(deserializer);
      case 8:
        return TypeTagU16.load(deserializer);
      case 9:
        return TypeTagU32.load(deserializer);
      case 10:
        return TypeTagU256.load(deserializer);
      default:
        throw new Error(`Unknown variant index for TypeTag: ${index}`);
    }
  }
}

export class TypeTagBool extends TypeTag {
  serialize(serializer: Serializer): void {
    serializer.serializeU32AsUleb128(0);
  }

  static load(deserializer: Deserializer): TypeTagBool {
    return new TypeTagBool();
  }
}

export class TypeTagU8 extends TypeTag {
  serialize(serializer: Serializer): void {
    serializer.serializeU32AsUleb128(1);
  }

  static load(_deserializer: Deserializer): TypeTagU8 {
    return new TypeTagU8();
  }
}

export class TypeTagU16 extends TypeTag {
  serialize(serializer: Serializer): void {
    serializer.serializeU32AsUleb128(1);
  }

  static load(_deserializer: Deserializer): TypeTagU16 {
    return new TypeTagU16();
  }
}

export class TypeTagU32 extends TypeTag {
  serialize(serializer: Serializer): void {
    serializer.serializeU32AsUleb128(1);
  }

  static load(_deserializer: Deserializer): TypeTagU32 {
    return new TypeTagU32();
  }
}

export class TypeTagU64 extends TypeTag {
  serialize(serializer: Serializer): void {
    serializer.serializeU32AsUleb128(2);
  }

  static load(_deserializer: Deserializer): TypeTagU64 {
    return new TypeTagU64();
  }
}

export class TypeTagU128 extends TypeTag {
  serialize(serializer: Serializer): void {
    serializer.serializeU32AsUleb128(3);
  }

  static load(_deserializer: Deserializer): TypeTagU128 {
    return new TypeTagU128();
  }
}

export class TypeTagU256 extends TypeTag {
  serialize(serializer: Serializer): void {
    serializer.serializeU32AsUleb128(1);
  }

  static load(_deserializer: Deserializer): TypeTagU256 {
    return new TypeTagU256();
  }
}

export class TypeTagAddress extends TypeTag {
  serialize(serializer: Serializer): void {
    serializer.serializeU32AsUleb128(4);
  }

  static load(_deserializer: Deserializer): TypeTagAddress {
    return new TypeTagAddress();
  }
}

export class TypeTagSigner extends TypeTag {
  serialize(serializer: Serializer): void {
    serializer.serializeU32AsUleb128(5);
  }

  static load(_deserializer: Deserializer): TypeTagSigner {
    return new TypeTagSigner();
  }
}

export class TypeTagVector extends TypeTag {
  constructor(public readonly value: TypeTag) {
    super();
  }

  serialize(serializer: Serializer): void {
    serializer.serializeU32AsUleb128(6);
    this.value.serialize(serializer);
  }

  static load(deserializer: Deserializer): TypeTagVector {
    const value = TypeTag.deserialize(deserializer);
    return new TypeTagVector(value);
  }
}

export class TypeTagStruct extends TypeTag {
  constructor(public readonly value: StructTag) {
    super();
  }

  serialize(serializer: Serializer): void {
    serializer.serializeU32AsUleb128(7);
    this.value.serialize(serializer);
  }

  static load(deserializer: Deserializer): TypeTagStruct {
    const value = StructTag.deserialize(deserializer);
    return new TypeTagStruct(value);
  }

  isStringTypeTag(): boolean {
    if (
      this.value.module_name.value === "string" &&
      this.value.name.value === "String" &&
      this.value.address.toHexString() === AccountAddress.fromHex("0x1").toHexString()
    ) {
      return true;
    }
    return false;
  }
}

export class StructTag {
  constructor(
    public readonly address: AccountAddress,
    public readonly module_name: Identifier,
    public readonly name: Identifier,
    public readonly type_args: Seq<TypeTag>,
  ) {}

  /**
   * Converts a string literal to a StructTag
   * @param structTag String literal in format "AcountAddress::module_name::ResourceName",
   *   e.g. "0x1::aptos_coin::AptosCoin"
   * @returns
   */
  static fromString(structTag: string): StructTag {
    // Use the TypeTagParser to parse the string literal into a TypeTagStruct
    const typeTagStruct = new TypeTagParser(structTag).parseTypeTag() as TypeTagStruct;

    // Convert and return as a StructTag
    return new StructTag(
      typeTagStruct.value.address,
      typeTagStruct.value.module_name,
      typeTagStruct.value.name,
      typeTagStruct.value.type_args,
    );
  }

  serialize(serializer: Serializer): void {
    this.address.serialize(serializer);
    this.module_name.serialize(serializer);
    this.name.serialize(serializer);
    serializeVector<TypeTag>(this.type_args, serializer);
  }

  static deserialize(deserializer: Deserializer): StructTag {
    const address = AccountAddress.deserialize(deserializer);
    const moduleName = Identifier.deserialize(deserializer);
    const name = Identifier.deserialize(deserializer);
    const typeArgs = deserializeVector(deserializer, TypeTag);
    return new StructTag(address, moduleName, name, typeArgs);
  }
}

export const stringStructTag = new StructTag(
  AccountAddress.fromHex("0x1"),
  new Identifier("string"),
  new Identifier("String"),
  [],
);

function bail(message: string) {
  throw new TypeTagParserError(message);
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

// Generic format is T<digits> - for example T1, T2, T10
function isGeneric(c: string): boolean {
  if (c.match(/T\d+/g)) {
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
    if (isGeneric(res)) {
      return [["GENERIC", res], res.length];
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

  private readonly typeTags: string[] = [];

  constructor(tagStr: string, typeTags?: string[]) {
    this.tokens = tokenize(tagStr);
    this.typeTags = typeTags || [];
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
    const [tokenTy, tokenVal] = this.tokens.shift()!;

    if (tokenVal === "u8") {
      return new TypeTagU8();
    }
    if (tokenVal === "u16") {
      return new TypeTagU16();
    }
    if (tokenVal === "u32") {
      return new TypeTagU32();
    }
    if (tokenVal === "u64") {
      return new TypeTagU64();
    }
    if (tokenVal === "u128") {
      return new TypeTagU128();
    }
    if (tokenVal === "u256") {
      return new TypeTagU256();
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
    if (tokenVal === "string") {
      return new StructTag(AccountAddress.fromHex("0x1"), new Identifier("string"), new Identifier("String"), []);
    }
    if (tokenTy === "IDENT" && (tokenVal.startsWith("0x") || tokenVal.startsWith("0X"))) {
      const address = tokenVal;
      this.consume("::");
      const [moduleTokenTy, module] = this.tokens.shift()!;
      if (moduleTokenTy !== "IDENT") {
        bail("Invalid type tag.");
      }
      this.consume("::");
      const [nameTokenTy, name] = this.tokens.shift()!;
      if (nameTokenTy !== "IDENT") {
        bail("Invalid type tag.");
      }

      // an Object `0x1::object::Object<T>` doesn't hold a real type, it points to an address
      // therefore, we parse it as an address and dont need to care/parse the `T` type
      if (module === "object" && name === "Object") {
        // to support a nested type tag, i.e 0x1::some_module::SomeResource<0x1::object::Object<T>>, we want
        // to remove the `<T>` part from the tokens list so we dont parse it and can keep parse the type tag.
        this.tokens.splice(0, 3);
        return new TypeTagAddress();
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
    if (tokenTy === "GENERIC") {
      if (this.typeTags.length === 0) {
        bail("Can't convert generic type since no typeTags were specified.");
      }
      // a generic tokenVal has the format of `T<digit>`, for example `T1`.
      // The digit (i.e 1) indicates the the index of this type in the typeTags array.
      // For a tokenVal == T1, should be parsed as the type in typeTags[1]
      const idx = parseInt(tokenVal.substring(1), 10);
      return new TypeTagParser(this.typeTags[idx]).parseTypeTag();
    }

    throw new Error("Invalid type tag.");
  }
}

export class TypeTagParserError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "TypeTagParserError";
  }
}
