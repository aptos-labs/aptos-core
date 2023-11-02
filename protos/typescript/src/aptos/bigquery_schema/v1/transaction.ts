/* eslint-disable */
import Long from "long";
import _m0 from "protobufjs/minimal";

/**
 * Proto2 is required.
 * Current BigQuery runs over proto2, thus optional(nullable)
 * field with default value will be ignored. For example,
 * `int64 value = null` will be translated to 0 under column `value`.
 * To avoid any analytics hassle, proto2 is required here.
 */

/**
 * Transaction is a simplified representation for the transaction
 * happened on the chain. Mainly built for streaming into BigQuery.
 * It matches with the structure defined for the transaction in Indexer.
 */
export interface Transaction {
  version?: bigint | undefined;
  blockHeight?: bigint | undefined;
  hash?: string | undefined;
  type?: string | undefined;
  payload?: string | undefined;
  stateChangeHash?: string | undefined;
  eventRootHash?: string | undefined;
  stateCheckpointHash?: string | undefined;
  gasUsed?: bigint | undefined;
  success?: boolean | undefined;
  vmStatus?: string | undefined;
  accumulatorRootHash?: string | undefined;
  numEvents?: bigint | undefined;
  numWriteSetChanges?: bigint | undefined;
  epoch?: bigint | undefined;
  insertedAt?: bigint | undefined;
}

function createBaseTransaction(): Transaction {
  return {
    version: BigInt("0"),
    blockHeight: BigInt("0"),
    hash: "",
    type: "",
    payload: "",
    stateChangeHash: "",
    eventRootHash: "",
    stateCheckpointHash: "",
    gasUsed: BigInt("0"),
    success: false,
    vmStatus: "",
    accumulatorRootHash: "",
    numEvents: BigInt("0"),
    numWriteSetChanges: BigInt("0"),
    epoch: BigInt("0"),
    insertedAt: BigInt("0"),
  };
}

export const Transaction = {
  encode(message: Transaction, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.version !== undefined && message.version !== BigInt("0")) {
      if (BigInt.asIntN(64, message.version) !== message.version) {
        throw new Error("value provided for field message.version of type int64 too large");
      }
      writer.uint32(8).int64(message.version.toString());
    }
    if (message.blockHeight !== undefined && message.blockHeight !== BigInt("0")) {
      if (BigInt.asIntN(64, message.blockHeight) !== message.blockHeight) {
        throw new Error("value provided for field message.blockHeight of type int64 too large");
      }
      writer.uint32(16).int64(message.blockHeight.toString());
    }
    if (message.hash !== undefined && message.hash !== "") {
      writer.uint32(26).string(message.hash);
    }
    if (message.type !== undefined && message.type !== "") {
      writer.uint32(34).string(message.type);
    }
    if (message.payload !== undefined && message.payload !== "") {
      writer.uint32(42).string(message.payload);
    }
    if (message.stateChangeHash !== undefined && message.stateChangeHash !== "") {
      writer.uint32(50).string(message.stateChangeHash);
    }
    if (message.eventRootHash !== undefined && message.eventRootHash !== "") {
      writer.uint32(58).string(message.eventRootHash);
    }
    if (message.stateCheckpointHash !== undefined && message.stateCheckpointHash !== "") {
      writer.uint32(66).string(message.stateCheckpointHash);
    }
    if (message.gasUsed !== undefined && message.gasUsed !== BigInt("0")) {
      if (BigInt.asUintN(64, message.gasUsed) !== message.gasUsed) {
        throw new Error("value provided for field message.gasUsed of type uint64 too large");
      }
      writer.uint32(72).uint64(message.gasUsed.toString());
    }
    if (message.success === true) {
      writer.uint32(80).bool(message.success);
    }
    if (message.vmStatus !== undefined && message.vmStatus !== "") {
      writer.uint32(90).string(message.vmStatus);
    }
    if (message.accumulatorRootHash !== undefined && message.accumulatorRootHash !== "") {
      writer.uint32(98).string(message.accumulatorRootHash);
    }
    if (message.numEvents !== undefined && message.numEvents !== BigInt("0")) {
      if (BigInt.asIntN(64, message.numEvents) !== message.numEvents) {
        throw new Error("value provided for field message.numEvents of type int64 too large");
      }
      writer.uint32(104).int64(message.numEvents.toString());
    }
    if (message.numWriteSetChanges !== undefined && message.numWriteSetChanges !== BigInt("0")) {
      if (BigInt.asIntN(64, message.numWriteSetChanges) !== message.numWriteSetChanges) {
        throw new Error("value provided for field message.numWriteSetChanges of type int64 too large");
      }
      writer.uint32(112).int64(message.numWriteSetChanges.toString());
    }
    if (message.epoch !== undefined && message.epoch !== BigInt("0")) {
      if (BigInt.asIntN(64, message.epoch) !== message.epoch) {
        throw new Error("value provided for field message.epoch of type int64 too large");
      }
      writer.uint32(120).int64(message.epoch.toString());
    }
    if (message.insertedAt !== undefined && message.insertedAt !== BigInt("0")) {
      if (BigInt.asIntN(64, message.insertedAt) !== message.insertedAt) {
        throw new Error("value provided for field message.insertedAt of type int64 too large");
      }
      writer.uint32(128).int64(message.insertedAt.toString());
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): Transaction {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseTransaction();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 8) {
            break;
          }

          message.version = longToBigint(reader.int64() as Long);
          continue;
        case 2:
          if (tag !== 16) {
            break;
          }

          message.blockHeight = longToBigint(reader.int64() as Long);
          continue;
        case 3:
          if (tag !== 26) {
            break;
          }

          message.hash = reader.string();
          continue;
        case 4:
          if (tag !== 34) {
            break;
          }

          message.type = reader.string();
          continue;
        case 5:
          if (tag !== 42) {
            break;
          }

          message.payload = reader.string();
          continue;
        case 6:
          if (tag !== 50) {
            break;
          }

          message.stateChangeHash = reader.string();
          continue;
        case 7:
          if (tag !== 58) {
            break;
          }

          message.eventRootHash = reader.string();
          continue;
        case 8:
          if (tag !== 66) {
            break;
          }

          message.stateCheckpointHash = reader.string();
          continue;
        case 9:
          if (tag !== 72) {
            break;
          }

          message.gasUsed = longToBigint(reader.uint64() as Long);
          continue;
        case 10:
          if (tag !== 80) {
            break;
          }

          message.success = reader.bool();
          continue;
        case 11:
          if (tag !== 90) {
            break;
          }

          message.vmStatus = reader.string();
          continue;
        case 12:
          if (tag !== 98) {
            break;
          }

          message.accumulatorRootHash = reader.string();
          continue;
        case 13:
          if (tag !== 104) {
            break;
          }

          message.numEvents = longToBigint(reader.int64() as Long);
          continue;
        case 14:
          if (tag !== 112) {
            break;
          }

          message.numWriteSetChanges = longToBigint(reader.int64() as Long);
          continue;
        case 15:
          if (tag !== 120) {
            break;
          }

          message.epoch = longToBigint(reader.int64() as Long);
          continue;
        case 16:
          if (tag !== 128) {
            break;
          }

          message.insertedAt = longToBigint(reader.int64() as Long);
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  // encodeTransform encodes a source of message objects.
  // Transform<Transaction, Uint8Array>
  async *encodeTransform(
    source: AsyncIterable<Transaction | Transaction[]> | Iterable<Transaction | Transaction[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [Transaction.encode(p).finish()];
        }
      } else {
        yield* [Transaction.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, Transaction>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<Transaction> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [Transaction.decode(p)];
        }
      } else {
        yield* [Transaction.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): Transaction {
    return {
      version: isSet(object.version) ? BigInt(object.version) : BigInt("0"),
      blockHeight: isSet(object.blockHeight) ? BigInt(object.blockHeight) : BigInt("0"),
      hash: isSet(object.hash) ? globalThis.String(object.hash) : "",
      type: isSet(object.type) ? globalThis.String(object.type) : "",
      payload: isSet(object.payload) ? globalThis.String(object.payload) : "",
      stateChangeHash: isSet(object.stateChangeHash) ? globalThis.String(object.stateChangeHash) : "",
      eventRootHash: isSet(object.eventRootHash) ? globalThis.String(object.eventRootHash) : "",
      stateCheckpointHash: isSet(object.stateCheckpointHash) ? globalThis.String(object.stateCheckpointHash) : "",
      gasUsed: isSet(object.gasUsed) ? BigInt(object.gasUsed) : BigInt("0"),
      success: isSet(object.success) ? globalThis.Boolean(object.success) : false,
      vmStatus: isSet(object.vmStatus) ? globalThis.String(object.vmStatus) : "",
      accumulatorRootHash: isSet(object.accumulatorRootHash) ? globalThis.String(object.accumulatorRootHash) : "",
      numEvents: isSet(object.numEvents) ? BigInt(object.numEvents) : BigInt("0"),
      numWriteSetChanges: isSet(object.numWriteSetChanges) ? BigInt(object.numWriteSetChanges) : BigInt("0"),
      epoch: isSet(object.epoch) ? BigInt(object.epoch) : BigInt("0"),
      insertedAt: isSet(object.insertedAt) ? BigInt(object.insertedAt) : BigInt("0"),
    };
  },

  toJSON(message: Transaction): unknown {
    const obj: any = {};
    if (message.version !== undefined && message.version !== BigInt("0")) {
      obj.version = message.version.toString();
    }
    if (message.blockHeight !== undefined && message.blockHeight !== BigInt("0")) {
      obj.blockHeight = message.blockHeight.toString();
    }
    if (message.hash !== undefined && message.hash !== "") {
      obj.hash = message.hash;
    }
    if (message.type !== undefined && message.type !== "") {
      obj.type = message.type;
    }
    if (message.payload !== undefined && message.payload !== "") {
      obj.payload = message.payload;
    }
    if (message.stateChangeHash !== undefined && message.stateChangeHash !== "") {
      obj.stateChangeHash = message.stateChangeHash;
    }
    if (message.eventRootHash !== undefined && message.eventRootHash !== "") {
      obj.eventRootHash = message.eventRootHash;
    }
    if (message.stateCheckpointHash !== undefined && message.stateCheckpointHash !== "") {
      obj.stateCheckpointHash = message.stateCheckpointHash;
    }
    if (message.gasUsed !== undefined && message.gasUsed !== BigInt("0")) {
      obj.gasUsed = message.gasUsed.toString();
    }
    if (message.success === true) {
      obj.success = message.success;
    }
    if (message.vmStatus !== undefined && message.vmStatus !== "") {
      obj.vmStatus = message.vmStatus;
    }
    if (message.accumulatorRootHash !== undefined && message.accumulatorRootHash !== "") {
      obj.accumulatorRootHash = message.accumulatorRootHash;
    }
    if (message.numEvents !== undefined && message.numEvents !== BigInt("0")) {
      obj.numEvents = message.numEvents.toString();
    }
    if (message.numWriteSetChanges !== undefined && message.numWriteSetChanges !== BigInt("0")) {
      obj.numWriteSetChanges = message.numWriteSetChanges.toString();
    }
    if (message.epoch !== undefined && message.epoch !== BigInt("0")) {
      obj.epoch = message.epoch.toString();
    }
    if (message.insertedAt !== undefined && message.insertedAt !== BigInt("0")) {
      obj.insertedAt = message.insertedAt.toString();
    }
    return obj;
  },

  create(base?: DeepPartial<Transaction>): Transaction {
    return Transaction.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<Transaction>): Transaction {
    const message = createBaseTransaction();
    message.version = object.version ?? BigInt("0");
    message.blockHeight = object.blockHeight ?? BigInt("0");
    message.hash = object.hash ?? "";
    message.type = object.type ?? "";
    message.payload = object.payload ?? "";
    message.stateChangeHash = object.stateChangeHash ?? "";
    message.eventRootHash = object.eventRootHash ?? "";
    message.stateCheckpointHash = object.stateCheckpointHash ?? "";
    message.gasUsed = object.gasUsed ?? BigInt("0");
    message.success = object.success ?? false;
    message.vmStatus = object.vmStatus ?? "";
    message.accumulatorRootHash = object.accumulatorRootHash ?? "";
    message.numEvents = object.numEvents ?? BigInt("0");
    message.numWriteSetChanges = object.numWriteSetChanges ?? BigInt("0");
    message.epoch = object.epoch ?? BigInt("0");
    message.insertedAt = object.insertedAt ?? BigInt("0");
    return message;
  },
};

type Builtin = Date | Function | Uint8Array | string | number | boolean | bigint | undefined;

type DeepPartial<T> = T extends Builtin ? T
  : T extends globalThis.Array<infer U> ? globalThis.Array<DeepPartial<U>>
  : T extends ReadonlyArray<infer U> ? ReadonlyArray<DeepPartial<U>>
  : T extends {} ? { [K in keyof T]?: DeepPartial<T[K]> }
  : Partial<T>;

function longToBigint(long: Long) {
  return BigInt(long.toString());
}

if (_m0.util.Long !== Long) {
  _m0.util.Long = Long as any;
  _m0.configure();
}

function isSet(value: any): boolean {
  return value !== null && value !== undefined;
}
