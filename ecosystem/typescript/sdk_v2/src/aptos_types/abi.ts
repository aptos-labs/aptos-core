// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

import { Deserializer, Serializer } from "../bcs";

import { ModuleId } from "./transaction";

import { TypeTag } from "./type_tag";

export class TypeArgumentABI {
  /**
   * Constructs a TypeArgumentABI instance.
   * @param name
   */
  constructor(public readonly name: string) {}

  serialize(serializer: Serializer): void {
    serializer.serializeStr(this.name);
  }

  static deserialize(deserializer: Deserializer): TypeArgumentABI {
    const name = deserializer.deserializeStr();
    return new TypeArgumentABI(name);
  }
}

export class ArgumentABI {
  /**
   * Constructs an ArgumentABI instance.
   * @param name
   * @param type_tag
   */
  constructor(public readonly name: string, public readonly type_tag: TypeTag) {}

  serialize(serializer: Serializer): void {
    serializer.serializeStr(this.name);
    this.type_tag.serialize(serializer);
  }

  static deserialize(deserializer: Deserializer): ArgumentABI {
    const name = deserializer.deserializeStr();
    const typeTag = TypeTag.deserialize(deserializer);
    return new ArgumentABI(name, typeTag);
  }
}

export abstract class ScriptABI {
  abstract serialize(serializer: Serializer): void;

  static deserialize(deserializer: Deserializer): ScriptABI {
    const index = deserializer.deserializeUleb128AsU32();
    switch (index) {
      case 0:
        return TransactionScriptABI.load(deserializer);
      case 1:
        return EntryFunctionABI.load(deserializer);
      default:
        throw new Error(`Unknown variant index for TransactionPayload: ${index}`);
    }
  }
}

export class TransactionScriptABI extends ScriptABI {
  /**
   * Constructs a TransactionScriptABI instance.
   * @param name Entry function name
   * @param doc
   * @param code
   * @param ty_args
   * @param args
   */
  constructor(
    public readonly name: string,
    public readonly doc: string,
    public readonly code: Uint8Array,
    public readonly ty_args: Array<TypeArgumentABI>,
    public readonly args: Array<ArgumentABI>,
  ) {
    super();
  }

  serialize(serializer: Serializer): void {
    serializer
        .serializeU32AsUleb128(0)
        .serializeStr(this.name)
        .serializeStr(this.doc)
        .serializeByteVector(this.code)
        .serializeVector<TypeArgumentABI>(this.ty_args)
        .serializeVector<ArgumentABI>(this.args)
  }

  static load(deserializer: Deserializer): TransactionScriptABI {
    const name = deserializer.deserializeStr();
    const doc = deserializer.deserializeStr();
    const code = deserializer.deserializeBytes();
    const tyArgs = deserializer.deserializeVector(TypeArgumentABI);
    const args = deserializer.deserializeVector(ArgumentABI);
    return new TransactionScriptABI(name, doc, code, tyArgs, args);
  }
}

export class EntryFunctionABI extends ScriptABI {
  /**
   * Constructs a EntryFunctionABI instance
   * @param name
   * @param module_name Fully qualified module id
   * @param doc
   * @param ty_args
   * @param args
   */
  constructor(
    public readonly name: string,
    public readonly module_name: ModuleId,
    public readonly doc: string,
    public readonly ty_args: Array<TypeArgumentABI>,
    public readonly args: Array<ArgumentABI>,
  ) {
    super();
  }

  serialize(serializer: Serializer): void {
    serializer
        .serializeU32AsUleb128(1)
        .serializeStr(this.name)
        .serialize(this.module_name)
        .serializeStr(this.doc)
        .serializeVector<TypeArgumentABI>(this.ty_args)
        .serializeVector<ArgumentABI>(this.args)
  }

  static load(deserializer: Deserializer): EntryFunctionABI {
    const name = deserializer.deserializeStr();
    const moduleName = ModuleId.deserialize(deserializer);
    const doc = deserializer.deserializeStr();
    const tyArgs = deserializer.deserializeVector(TypeArgumentABI);
    const args = deserializer.deserializeVector(ArgumentABI);
    return new EntryFunctionABI(name, moduleName, doc, tyArgs, args);
  }
}
