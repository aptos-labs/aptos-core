// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

import { AccountAddress } from "../../core";
import { Identifier } from "./identifier";
import {
  TypeTag,
  TypeTagU8,
  TypeTagU16,
  TypeTagU32,
  TypeTagU64,
  TypeTagU128,
  TypeTagU256,
  TypeTagBool,
  TypeTagAddress,
  TypeTagVector,
  TypeTagStruct,
  stringStructTag,
  StructTag,
} from "./typeTag";

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

  /**
   * Consumes all of an unused generic field, mostly applicable to object
   *
   * Note: This is recursive.  it can be problematic if there's bad input
   * @private
   */
  private consumeWholeGeneric() {
    this.consume("<");
    while (this.tokens[0][1] !== ">") {
      // If it is nested, we have to consume another nested generic
      if (this.tokens[0][1] === "<") {
        this.consumeWholeGeneric();
      }
      this.tokens.shift();
    }
    this.consume(">");
  }

  private parseCommaList(endToken: string, allowTraillingComma: boolean): TypeTag[] {
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
      return new TypeTagStruct(stringStructTag);
    }
    if (tokenTy === "IDENT" && (tokenVal.startsWith("0x") || tokenVal.startsWith("0X"))) {
      const address = AccountAddress.fromHexInput({ input: tokenVal });
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

      // Objects can contain either concrete types e.g. 0x1::object::ObjectCore or generics e.g. T
      // Neither matter as we can't do type checks, so just the address applies and we consume the entire generic.
      // TODO: Support parsing structs that don't come from core code address
      if (AccountAddress.ONE.toString() === address.toString() && module === "object" && name === "Object") {
        this.consumeWholeGeneric();
        return new TypeTagAddress();
      }

      let tyTags: TypeTag[] = [];
      // Check if the struct has ty args
      if (this.tokens.length > 0 && this.tokens[0][1] === "<") {
        this.consume("<");
        tyTags = this.parseCommaList(">", true);
        this.consume(">");
      }

      const structTag = new StructTag(address, new Identifier(module), new Identifier(name), tyTags);
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

/**
 * Used for parsing a TypeTag, a Token type is two strings: [token type, token value]
 * @example const token: Token = ["COMMA", ","];
 * @see nextToken(...) in typeTagParser.ts
 */
type Token = [string, string];

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
