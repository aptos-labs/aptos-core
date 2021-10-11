
import { Serializer, Deserializer } from '../serde/mod.ts';
import { BcsSerializer, BcsDeserializer } from '../bcs/mod.ts';
import { Optional, Seq, Tuple, ListTuple, unit, bool, int8, int16, int32, int64, int128, uint8, uint16, uint32, uint64, uint128, float32, float64, char, str, bytes } from '../serde/mod.ts';

import * as DiemTypes from '../diemTypes/mod.ts';

/**
 * Structured representation of a call into a known Move script.
 */
export abstract class ScriptCall {
}


export class ScriptCallVariantSetMessage extends ScriptCall {

constructor (public message: bytes) {
  super();
}

}

export interface TypeTagDef {
  type: Types;
  arrayType?: TypeTagDef;
  name?: string;
  moduleName?: string;
  address?: string;
  typeParams?: TypeTagDef[];
}

export interface ArgDef {
  readonly name: string;
  readonly type: TypeTagDef;
  readonly choices?: string[];
  readonly mandatory?: boolean;
}

export interface ScriptDef {
  readonly stdlibEncodeFunction: (...args: any[]) => DiemTypes.Script;
  readonly stdlibDecodeFunction: (script: DiemTypes.Script) => ScriptCall;
  readonly codeName: string;
  readonly description: string;
  readonly typeArgs: string[];
  readonly args: ArgDef[];
}

export enum Types {
  Boolean,
  U8,
  U64,
  U128,
  Address,
  Array,
  Struct
}


export class Stdlib {
  private static fromHexString(hexString: string): Uint8Array { return new Uint8Array(hexString.match(/.{1,2}/g)!.map((byte) => parseInt(byte, 16)));}

  /**

   */
  static encodeSetMessageScript(message: Uint8Array): DiemTypes.Script {
    const code = Stdlib.SET_MESSAGE_CODE;
    const tyArgs: Seq<DiemTypes.TypeTag> = [];
    const args: Seq<DiemTypes.TransactionArgument> = [new DiemTypes.TransactionArgumentVariantU8Vector(message)];
    return new DiemTypes.Script(code, tyArgs, args);
  }

  static decodeSetMessageScript(script: DiemTypes.Script): ScriptCallVariantSetMessage {
    return new ScriptCallVariantSetMessage(
      (script.args[0] as DiemTypes.TransactionArgumentVariantU8Vector).value
    );
  }

  static SET_MESSAGE_CODE = Stdlib.fromHexString('a11ceb0b0300000005010002030205050705070c1408201000000001000100020c0a0200074d6573736167650b7365745f6d65737361676524163afcc6e33b0a9473852e18327fa9000001040b000b01110002');

  static ScriptArgs: {[name: string]: ScriptDef} = {
    SetMessage: {
      stdlibEncodeFunction: Stdlib.encodeSetMessageScript,
      stdlibDecodeFunction: Stdlib.decodeSetMessageScript,
      codeName: 'SET_MESSAGE',
      description: "",
      typeArgs: [],
      args: [
    {name: "message", type: {type: Types.Array, arrayType: {type: Types.U8}}}
      ]
    },
  }

}


export type ScriptDecoders = {
  User: {
    SetMessage: (type: string, message: DiemTypes.TransactionArgumentVariantU8Vector) => void;
    default: (type: keyof ScriptDecoders['User']) => void;
  };
};
