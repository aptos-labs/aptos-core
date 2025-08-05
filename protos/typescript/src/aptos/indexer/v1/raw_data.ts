/* eslint-disable */
import {
  ChannelCredentials,
  Client,
  ClientReadableStream,
  handleServerStreamingCall,
  makeGenericClientConstructor,
  Metadata,
} from "@grpc/grpc-js";
import type { CallOptions, ClientOptions, UntypedServiceImplementation } from "@grpc/grpc-js";
import Long from "long";
import _m0 from "protobufjs/minimal";
import { Event, Transaction } from "../../transaction/v1/transaction";
import { Timestamp } from "../../util/timestamp/timestamp";
import { BooleanTransactionFilter } from "./filter";

/** This is for storage only. */
export interface TransactionsInStorage {
  /** Required; transactions data. */
  transactions?:
    | Transaction[]
    | undefined;
  /** Required; chain id. */
  startingVersion?: bigint | undefined;
}

export interface GetTransactionsRequest {
  /** Required; start version of current stream. */
  startingVersion?:
    | bigint
    | undefined;
  /**
   * Optional; number of transactions to return in current stream.
   * If not present, return an infinite stream of transactions.
   */
  transactionsCount?:
    | bigint
    | undefined;
  /**
   * Optional; number of transactions in each `TransactionsResponse` for current stream.
   * If not present, default to 1000. If larger than 1000, request will be rejected.
   */
  batchSize?:
    | bigint
    | undefined;
  /** If provided, only transactions that match the filter will be included. */
  transactionFilter?: BooleanTransactionFilter | undefined;
}

export interface ProcessedRange {
  firstVersion?: bigint | undefined;
  lastVersion?: bigint | undefined;
}

/** TransactionsResponse is a batch of transactions. */
export interface TransactionsResponse {
  /** Required; transactions data. */
  transactions?:
    | Transaction[]
    | undefined;
  /** Required; chain id. */
  chainId?: bigint | undefined;
  processedRange?: ProcessedRange | undefined;
}

/** EventWithMetadata combines Event data with key transaction metadata. */
export interface EventWithMetadata {
  /** Required; the event data. */
  event?:
    | Event
    | undefined;
  /** Required; transaction metadata. */
  timestamp?: Timestamp | undefined;
  version?: bigint | undefined;
  hash?: Uint8Array | undefined;
  success?: boolean | undefined;
  vmStatus?: string | undefined;
  blockHeight?: bigint | undefined;
}

export interface GetEventsRequest {
  /** Required; start version of current stream. */
  startingVersion?:
    | bigint
    | undefined;
  /**
   * Optional; number of transactions to process in current stream.
   * If not present, return an infinite stream of events.
   */
  transactionsCount?:
    | bigint
    | undefined;
  /**
   * Optional; number of events in each `EventsResponse` for current stream.
   * If not present, default to 1000. If larger than 1000, request will be rejected.
   */
  batchSize?:
    | bigint
    | undefined;
  /**
   * If provided, only transactions that match the filter will be included,
   * and only events from those transactions will be returned.
   */
  transactionFilter?: BooleanTransactionFilter | undefined;
}

export interface EventsResponse {
  /** Required; events data with metadata. */
  events?:
    | EventWithMetadata[]
    | undefined;
  /** Required; chain id. */
  chainId?: bigint | undefined;
  processedRange?: ProcessedRange | undefined;
}

function createBaseTransactionsInStorage(): TransactionsInStorage {
  return { transactions: [], startingVersion: undefined };
}

export const TransactionsInStorage = {
  encode(message: TransactionsInStorage, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.transactions !== undefined && message.transactions.length !== 0) {
      for (const v of message.transactions) {
        Transaction.encode(v!, writer.uint32(10).fork()).ldelim();
      }
    }
    if (message.startingVersion !== undefined) {
      if (BigInt.asUintN(64, message.startingVersion) !== message.startingVersion) {
        throw new globalThis.Error("value provided for field message.startingVersion of type uint64 too large");
      }
      writer.uint32(16).uint64(message.startingVersion.toString());
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): TransactionsInStorage {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseTransactionsInStorage();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.transactions!.push(Transaction.decode(reader, reader.uint32()));
          continue;
        case 2:
          if (tag !== 16) {
            break;
          }

          message.startingVersion = longToBigint(reader.uint64() as Long);
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
  // Transform<TransactionsInStorage, Uint8Array>
  async *encodeTransform(
    source:
      | AsyncIterable<TransactionsInStorage | TransactionsInStorage[]>
      | Iterable<TransactionsInStorage | TransactionsInStorage[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [TransactionsInStorage.encode(p).finish()];
        }
      } else {
        yield* [TransactionsInStorage.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, TransactionsInStorage>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<TransactionsInStorage> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [TransactionsInStorage.decode(p)];
        }
      } else {
        yield* [TransactionsInStorage.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): TransactionsInStorage {
    return {
      transactions: globalThis.Array.isArray(object?.transactions)
        ? object.transactions.map((e: any) => Transaction.fromJSON(e))
        : [],
      startingVersion: isSet(object.startingVersion) ? BigInt(object.startingVersion) : undefined,
    };
  },

  toJSON(message: TransactionsInStorage): unknown {
    const obj: any = {};
    if (message.transactions?.length) {
      obj.transactions = message.transactions.map((e) => Transaction.toJSON(e));
    }
    if (message.startingVersion !== undefined) {
      obj.startingVersion = message.startingVersion.toString();
    }
    return obj;
  },

  create(base?: DeepPartial<TransactionsInStorage>): TransactionsInStorage {
    return TransactionsInStorage.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<TransactionsInStorage>): TransactionsInStorage {
    const message = createBaseTransactionsInStorage();
    message.transactions = object.transactions?.map((e) => Transaction.fromPartial(e)) || [];
    message.startingVersion = object.startingVersion ?? undefined;
    return message;
  },
};

function createBaseGetTransactionsRequest(): GetTransactionsRequest {
  return {
    startingVersion: undefined,
    transactionsCount: undefined,
    batchSize: undefined,
    transactionFilter: undefined,
  };
}

export const GetTransactionsRequest = {
  encode(message: GetTransactionsRequest, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.startingVersion !== undefined) {
      if (BigInt.asUintN(64, message.startingVersion) !== message.startingVersion) {
        throw new globalThis.Error("value provided for field message.startingVersion of type uint64 too large");
      }
      writer.uint32(8).uint64(message.startingVersion.toString());
    }
    if (message.transactionsCount !== undefined) {
      if (BigInt.asUintN(64, message.transactionsCount) !== message.transactionsCount) {
        throw new globalThis.Error("value provided for field message.transactionsCount of type uint64 too large");
      }
      writer.uint32(16).uint64(message.transactionsCount.toString());
    }
    if (message.batchSize !== undefined) {
      if (BigInt.asUintN(64, message.batchSize) !== message.batchSize) {
        throw new globalThis.Error("value provided for field message.batchSize of type uint64 too large");
      }
      writer.uint32(24).uint64(message.batchSize.toString());
    }
    if (message.transactionFilter !== undefined) {
      BooleanTransactionFilter.encode(message.transactionFilter, writer.uint32(34).fork()).ldelim();
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): GetTransactionsRequest {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseGetTransactionsRequest();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 8) {
            break;
          }

          message.startingVersion = longToBigint(reader.uint64() as Long);
          continue;
        case 2:
          if (tag !== 16) {
            break;
          }

          message.transactionsCount = longToBigint(reader.uint64() as Long);
          continue;
        case 3:
          if (tag !== 24) {
            break;
          }

          message.batchSize = longToBigint(reader.uint64() as Long);
          continue;
        case 4:
          if (tag !== 34) {
            break;
          }

          message.transactionFilter = BooleanTransactionFilter.decode(reader, reader.uint32());
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
  // Transform<GetTransactionsRequest, Uint8Array>
  async *encodeTransform(
    source:
      | AsyncIterable<GetTransactionsRequest | GetTransactionsRequest[]>
      | Iterable<GetTransactionsRequest | GetTransactionsRequest[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [GetTransactionsRequest.encode(p).finish()];
        }
      } else {
        yield* [GetTransactionsRequest.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, GetTransactionsRequest>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<GetTransactionsRequest> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [GetTransactionsRequest.decode(p)];
        }
      } else {
        yield* [GetTransactionsRequest.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): GetTransactionsRequest {
    return {
      startingVersion: isSet(object.startingVersion) ? BigInt(object.startingVersion) : undefined,
      transactionsCount: isSet(object.transactionsCount) ? BigInt(object.transactionsCount) : undefined,
      batchSize: isSet(object.batchSize) ? BigInt(object.batchSize) : undefined,
      transactionFilter: isSet(object.transactionFilter)
        ? BooleanTransactionFilter.fromJSON(object.transactionFilter)
        : undefined,
    };
  },

  toJSON(message: GetTransactionsRequest): unknown {
    const obj: any = {};
    if (message.startingVersion !== undefined) {
      obj.startingVersion = message.startingVersion.toString();
    }
    if (message.transactionsCount !== undefined) {
      obj.transactionsCount = message.transactionsCount.toString();
    }
    if (message.batchSize !== undefined) {
      obj.batchSize = message.batchSize.toString();
    }
    if (message.transactionFilter !== undefined) {
      obj.transactionFilter = BooleanTransactionFilter.toJSON(message.transactionFilter);
    }
    return obj;
  },

  create(base?: DeepPartial<GetTransactionsRequest>): GetTransactionsRequest {
    return GetTransactionsRequest.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<GetTransactionsRequest>): GetTransactionsRequest {
    const message = createBaseGetTransactionsRequest();
    message.startingVersion = object.startingVersion ?? undefined;
    message.transactionsCount = object.transactionsCount ?? undefined;
    message.batchSize = object.batchSize ?? undefined;
    message.transactionFilter = (object.transactionFilter !== undefined && object.transactionFilter !== null)
      ? BooleanTransactionFilter.fromPartial(object.transactionFilter)
      : undefined;
    return message;
  },
};

function createBaseProcessedRange(): ProcessedRange {
  return { firstVersion: BigInt("0"), lastVersion: BigInt("0") };
}

export const ProcessedRange = {
  encode(message: ProcessedRange, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.firstVersion !== undefined && message.firstVersion !== BigInt("0")) {
      if (BigInt.asUintN(64, message.firstVersion) !== message.firstVersion) {
        throw new globalThis.Error("value provided for field message.firstVersion of type uint64 too large");
      }
      writer.uint32(8).uint64(message.firstVersion.toString());
    }
    if (message.lastVersion !== undefined && message.lastVersion !== BigInt("0")) {
      if (BigInt.asUintN(64, message.lastVersion) !== message.lastVersion) {
        throw new globalThis.Error("value provided for field message.lastVersion of type uint64 too large");
      }
      writer.uint32(16).uint64(message.lastVersion.toString());
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): ProcessedRange {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseProcessedRange();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 8) {
            break;
          }

          message.firstVersion = longToBigint(reader.uint64() as Long);
          continue;
        case 2:
          if (tag !== 16) {
            break;
          }

          message.lastVersion = longToBigint(reader.uint64() as Long);
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
  // Transform<ProcessedRange, Uint8Array>
  async *encodeTransform(
    source: AsyncIterable<ProcessedRange | ProcessedRange[]> | Iterable<ProcessedRange | ProcessedRange[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [ProcessedRange.encode(p).finish()];
        }
      } else {
        yield* [ProcessedRange.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, ProcessedRange>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<ProcessedRange> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [ProcessedRange.decode(p)];
        }
      } else {
        yield* [ProcessedRange.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): ProcessedRange {
    return {
      firstVersion: isSet(object.firstVersion) ? BigInt(object.firstVersion) : BigInt("0"),
      lastVersion: isSet(object.lastVersion) ? BigInt(object.lastVersion) : BigInt("0"),
    };
  },

  toJSON(message: ProcessedRange): unknown {
    const obj: any = {};
    if (message.firstVersion !== undefined && message.firstVersion !== BigInt("0")) {
      obj.firstVersion = message.firstVersion.toString();
    }
    if (message.lastVersion !== undefined && message.lastVersion !== BigInt("0")) {
      obj.lastVersion = message.lastVersion.toString();
    }
    return obj;
  },

  create(base?: DeepPartial<ProcessedRange>): ProcessedRange {
    return ProcessedRange.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<ProcessedRange>): ProcessedRange {
    const message = createBaseProcessedRange();
    message.firstVersion = object.firstVersion ?? BigInt("0");
    message.lastVersion = object.lastVersion ?? BigInt("0");
    return message;
  },
};

function createBaseTransactionsResponse(): TransactionsResponse {
  return { transactions: [], chainId: undefined, processedRange: undefined };
}

export const TransactionsResponse = {
  encode(message: TransactionsResponse, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.transactions !== undefined && message.transactions.length !== 0) {
      for (const v of message.transactions) {
        Transaction.encode(v!, writer.uint32(10).fork()).ldelim();
      }
    }
    if (message.chainId !== undefined) {
      if (BigInt.asUintN(64, message.chainId) !== message.chainId) {
        throw new globalThis.Error("value provided for field message.chainId of type uint64 too large");
      }
      writer.uint32(16).uint64(message.chainId.toString());
    }
    if (message.processedRange !== undefined) {
      ProcessedRange.encode(message.processedRange, writer.uint32(26).fork()).ldelim();
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): TransactionsResponse {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseTransactionsResponse();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.transactions!.push(Transaction.decode(reader, reader.uint32()));
          continue;
        case 2:
          if (tag !== 16) {
            break;
          }

          message.chainId = longToBigint(reader.uint64() as Long);
          continue;
        case 3:
          if (tag !== 26) {
            break;
          }

          message.processedRange = ProcessedRange.decode(reader, reader.uint32());
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
  // Transform<TransactionsResponse, Uint8Array>
  async *encodeTransform(
    source:
      | AsyncIterable<TransactionsResponse | TransactionsResponse[]>
      | Iterable<TransactionsResponse | TransactionsResponse[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [TransactionsResponse.encode(p).finish()];
        }
      } else {
        yield* [TransactionsResponse.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, TransactionsResponse>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<TransactionsResponse> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [TransactionsResponse.decode(p)];
        }
      } else {
        yield* [TransactionsResponse.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): TransactionsResponse {
    return {
      transactions: globalThis.Array.isArray(object?.transactions)
        ? object.transactions.map((e: any) => Transaction.fromJSON(e))
        : [],
      chainId: isSet(object.chainId) ? BigInt(object.chainId) : undefined,
      processedRange: isSet(object.processedRange) ? ProcessedRange.fromJSON(object.processedRange) : undefined,
    };
  },

  toJSON(message: TransactionsResponse): unknown {
    const obj: any = {};
    if (message.transactions?.length) {
      obj.transactions = message.transactions.map((e) => Transaction.toJSON(e));
    }
    if (message.chainId !== undefined) {
      obj.chainId = message.chainId.toString();
    }
    if (message.processedRange !== undefined) {
      obj.processedRange = ProcessedRange.toJSON(message.processedRange);
    }
    return obj;
  },

  create(base?: DeepPartial<TransactionsResponse>): TransactionsResponse {
    return TransactionsResponse.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<TransactionsResponse>): TransactionsResponse {
    const message = createBaseTransactionsResponse();
    message.transactions = object.transactions?.map((e) => Transaction.fromPartial(e)) || [];
    message.chainId = object.chainId ?? undefined;
    message.processedRange = (object.processedRange !== undefined && object.processedRange !== null)
      ? ProcessedRange.fromPartial(object.processedRange)
      : undefined;
    return message;
  },
};

function createBaseEventWithMetadata(): EventWithMetadata {
  return {
    event: undefined,
    timestamp: undefined,
    version: BigInt("0"),
    hash: new Uint8Array(0),
    success: false,
    vmStatus: "",
    blockHeight: BigInt("0"),
  };
}

export const EventWithMetadata = {
  encode(message: EventWithMetadata, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.event !== undefined) {
      Event.encode(message.event, writer.uint32(10).fork()).ldelim();
    }
    if (message.timestamp !== undefined) {
      Timestamp.encode(message.timestamp, writer.uint32(18).fork()).ldelim();
    }
    if (message.version !== undefined && message.version !== BigInt("0")) {
      if (BigInt.asUintN(64, message.version) !== message.version) {
        throw new globalThis.Error("value provided for field message.version of type uint64 too large");
      }
      writer.uint32(24).uint64(message.version.toString());
    }
    if (message.hash !== undefined && message.hash.length !== 0) {
      writer.uint32(34).bytes(message.hash);
    }
    if (message.success === true) {
      writer.uint32(40).bool(message.success);
    }
    if (message.vmStatus !== undefined && message.vmStatus !== "") {
      writer.uint32(50).string(message.vmStatus);
    }
    if (message.blockHeight !== undefined && message.blockHeight !== BigInt("0")) {
      if (BigInt.asUintN(64, message.blockHeight) !== message.blockHeight) {
        throw new globalThis.Error("value provided for field message.blockHeight of type uint64 too large");
      }
      writer.uint32(56).uint64(message.blockHeight.toString());
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): EventWithMetadata {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseEventWithMetadata();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.event = Event.decode(reader, reader.uint32());
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.timestamp = Timestamp.decode(reader, reader.uint32());
          continue;
        case 3:
          if (tag !== 24) {
            break;
          }

          message.version = longToBigint(reader.uint64() as Long);
          continue;
        case 4:
          if (tag !== 34) {
            break;
          }

          message.hash = reader.bytes();
          continue;
        case 5:
          if (tag !== 40) {
            break;
          }

          message.success = reader.bool();
          continue;
        case 6:
          if (tag !== 50) {
            break;
          }

          message.vmStatus = reader.string();
          continue;
        case 7:
          if (tag !== 56) {
            break;
          }

          message.blockHeight = longToBigint(reader.uint64() as Long);
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
  // Transform<EventWithMetadata, Uint8Array>
  async *encodeTransform(
    source: AsyncIterable<EventWithMetadata | EventWithMetadata[]> | Iterable<EventWithMetadata | EventWithMetadata[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [EventWithMetadata.encode(p).finish()];
        }
      } else {
        yield* [EventWithMetadata.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, EventWithMetadata>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<EventWithMetadata> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [EventWithMetadata.decode(p)];
        }
      } else {
        yield* [EventWithMetadata.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): EventWithMetadata {
    return {
      event: isSet(object.event) ? Event.fromJSON(object.event) : undefined,
      timestamp: isSet(object.timestamp) ? Timestamp.fromJSON(object.timestamp) : undefined,
      version: isSet(object.version) ? BigInt(object.version) : BigInt("0"),
      hash: isSet(object.hash) ? bytesFromBase64(object.hash) : new Uint8Array(0),
      success: isSet(object.success) ? globalThis.Boolean(object.success) : false,
      vmStatus: isSet(object.vmStatus) ? globalThis.String(object.vmStatus) : "",
      blockHeight: isSet(object.blockHeight) ? BigInt(object.blockHeight) : BigInt("0"),
    };
  },

  toJSON(message: EventWithMetadata): unknown {
    const obj: any = {};
    if (message.event !== undefined) {
      obj.event = Event.toJSON(message.event);
    }
    if (message.timestamp !== undefined) {
      obj.timestamp = Timestamp.toJSON(message.timestamp);
    }
    if (message.version !== undefined && message.version !== BigInt("0")) {
      obj.version = message.version.toString();
    }
    if (message.hash !== undefined && message.hash.length !== 0) {
      obj.hash = base64FromBytes(message.hash);
    }
    if (message.success === true) {
      obj.success = message.success;
    }
    if (message.vmStatus !== undefined && message.vmStatus !== "") {
      obj.vmStatus = message.vmStatus;
    }
    if (message.blockHeight !== undefined && message.blockHeight !== BigInt("0")) {
      obj.blockHeight = message.blockHeight.toString();
    }
    return obj;
  },

  create(base?: DeepPartial<EventWithMetadata>): EventWithMetadata {
    return EventWithMetadata.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<EventWithMetadata>): EventWithMetadata {
    const message = createBaseEventWithMetadata();
    message.event = (object.event !== undefined && object.event !== null) ? Event.fromPartial(object.event) : undefined;
    message.timestamp = (object.timestamp !== undefined && object.timestamp !== null)
      ? Timestamp.fromPartial(object.timestamp)
      : undefined;
    message.version = object.version ?? BigInt("0");
    message.hash = object.hash ?? new Uint8Array(0);
    message.success = object.success ?? false;
    message.vmStatus = object.vmStatus ?? "";
    message.blockHeight = object.blockHeight ?? BigInt("0");
    return message;
  },
};

function createBaseGetEventsRequest(): GetEventsRequest {
  return {
    startingVersion: undefined,
    transactionsCount: undefined,
    batchSize: undefined,
    transactionFilter: undefined,
  };
}

export const GetEventsRequest = {
  encode(message: GetEventsRequest, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.startingVersion !== undefined) {
      if (BigInt.asUintN(64, message.startingVersion) !== message.startingVersion) {
        throw new globalThis.Error("value provided for field message.startingVersion of type uint64 too large");
      }
      writer.uint32(8).uint64(message.startingVersion.toString());
    }
    if (message.transactionsCount !== undefined) {
      if (BigInt.asUintN(64, message.transactionsCount) !== message.transactionsCount) {
        throw new globalThis.Error("value provided for field message.transactionsCount of type uint64 too large");
      }
      writer.uint32(16).uint64(message.transactionsCount.toString());
    }
    if (message.batchSize !== undefined) {
      if (BigInt.asUintN(64, message.batchSize) !== message.batchSize) {
        throw new globalThis.Error("value provided for field message.batchSize of type uint64 too large");
      }
      writer.uint32(24).uint64(message.batchSize.toString());
    }
    if (message.transactionFilter !== undefined) {
      BooleanTransactionFilter.encode(message.transactionFilter, writer.uint32(34).fork()).ldelim();
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): GetEventsRequest {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseGetEventsRequest();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 8) {
            break;
          }

          message.startingVersion = longToBigint(reader.uint64() as Long);
          continue;
        case 2:
          if (tag !== 16) {
            break;
          }

          message.transactionsCount = longToBigint(reader.uint64() as Long);
          continue;
        case 3:
          if (tag !== 24) {
            break;
          }

          message.batchSize = longToBigint(reader.uint64() as Long);
          continue;
        case 4:
          if (tag !== 34) {
            break;
          }

          message.transactionFilter = BooleanTransactionFilter.decode(reader, reader.uint32());
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
  // Transform<GetEventsRequest, Uint8Array>
  async *encodeTransform(
    source: AsyncIterable<GetEventsRequest | GetEventsRequest[]> | Iterable<GetEventsRequest | GetEventsRequest[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [GetEventsRequest.encode(p).finish()];
        }
      } else {
        yield* [GetEventsRequest.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, GetEventsRequest>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<GetEventsRequest> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [GetEventsRequest.decode(p)];
        }
      } else {
        yield* [GetEventsRequest.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): GetEventsRequest {
    return {
      startingVersion: isSet(object.startingVersion) ? BigInt(object.startingVersion) : undefined,
      transactionsCount: isSet(object.transactionsCount) ? BigInt(object.transactionsCount) : undefined,
      batchSize: isSet(object.batchSize) ? BigInt(object.batchSize) : undefined,
      transactionFilter: isSet(object.transactionFilter)
        ? BooleanTransactionFilter.fromJSON(object.transactionFilter)
        : undefined,
    };
  },

  toJSON(message: GetEventsRequest): unknown {
    const obj: any = {};
    if (message.startingVersion !== undefined) {
      obj.startingVersion = message.startingVersion.toString();
    }
    if (message.transactionsCount !== undefined) {
      obj.transactionsCount = message.transactionsCount.toString();
    }
    if (message.batchSize !== undefined) {
      obj.batchSize = message.batchSize.toString();
    }
    if (message.transactionFilter !== undefined) {
      obj.transactionFilter = BooleanTransactionFilter.toJSON(message.transactionFilter);
    }
    return obj;
  },

  create(base?: DeepPartial<GetEventsRequest>): GetEventsRequest {
    return GetEventsRequest.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<GetEventsRequest>): GetEventsRequest {
    const message = createBaseGetEventsRequest();
    message.startingVersion = object.startingVersion ?? undefined;
    message.transactionsCount = object.transactionsCount ?? undefined;
    message.batchSize = object.batchSize ?? undefined;
    message.transactionFilter = (object.transactionFilter !== undefined && object.transactionFilter !== null)
      ? BooleanTransactionFilter.fromPartial(object.transactionFilter)
      : undefined;
    return message;
  },
};

function createBaseEventsResponse(): EventsResponse {
  return { events: [], chainId: undefined, processedRange: undefined };
}

export const EventsResponse = {
  encode(message: EventsResponse, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.events !== undefined && message.events.length !== 0) {
      for (const v of message.events) {
        EventWithMetadata.encode(v!, writer.uint32(10).fork()).ldelim();
      }
    }
    if (message.chainId !== undefined) {
      if (BigInt.asUintN(64, message.chainId) !== message.chainId) {
        throw new globalThis.Error("value provided for field message.chainId of type uint64 too large");
      }
      writer.uint32(16).uint64(message.chainId.toString());
    }
    if (message.processedRange !== undefined) {
      ProcessedRange.encode(message.processedRange, writer.uint32(26).fork()).ldelim();
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): EventsResponse {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseEventsResponse();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.events!.push(EventWithMetadata.decode(reader, reader.uint32()));
          continue;
        case 2:
          if (tag !== 16) {
            break;
          }

          message.chainId = longToBigint(reader.uint64() as Long);
          continue;
        case 3:
          if (tag !== 26) {
            break;
          }

          message.processedRange = ProcessedRange.decode(reader, reader.uint32());
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
  // Transform<EventsResponse, Uint8Array>
  async *encodeTransform(
    source: AsyncIterable<EventsResponse | EventsResponse[]> | Iterable<EventsResponse | EventsResponse[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [EventsResponse.encode(p).finish()];
        }
      } else {
        yield* [EventsResponse.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, EventsResponse>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<EventsResponse> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [EventsResponse.decode(p)];
        }
      } else {
        yield* [EventsResponse.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): EventsResponse {
    return {
      events: globalThis.Array.isArray(object?.events)
        ? object.events.map((e: any) => EventWithMetadata.fromJSON(e))
        : [],
      chainId: isSet(object.chainId) ? BigInt(object.chainId) : undefined,
      processedRange: isSet(object.processedRange) ? ProcessedRange.fromJSON(object.processedRange) : undefined,
    };
  },

  toJSON(message: EventsResponse): unknown {
    const obj: any = {};
    if (message.events?.length) {
      obj.events = message.events.map((e) => EventWithMetadata.toJSON(e));
    }
    if (message.chainId !== undefined) {
      obj.chainId = message.chainId.toString();
    }
    if (message.processedRange !== undefined) {
      obj.processedRange = ProcessedRange.toJSON(message.processedRange);
    }
    return obj;
  },

  create(base?: DeepPartial<EventsResponse>): EventsResponse {
    return EventsResponse.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<EventsResponse>): EventsResponse {
    const message = createBaseEventsResponse();
    message.events = object.events?.map((e) => EventWithMetadata.fromPartial(e)) || [];
    message.chainId = object.chainId ?? undefined;
    message.processedRange = (object.processedRange !== undefined && object.processedRange !== null)
      ? ProcessedRange.fromPartial(object.processedRange)
      : undefined;
    return message;
  },
};

export type RawDataService = typeof RawDataService;
export const RawDataService = {
  /** Get transactions batch without any filtering from starting version and end if transaction count is present. */
  getTransactions: {
    path: "/aptos.indexer.v1.RawData/GetTransactions",
    requestStream: false,
    responseStream: true,
    requestSerialize: (value: GetTransactionsRequest) => Buffer.from(GetTransactionsRequest.encode(value).finish()),
    requestDeserialize: (value: Buffer) => GetTransactionsRequest.decode(value),
    responseSerialize: (value: TransactionsResponse) => Buffer.from(TransactionsResponse.encode(value).finish()),
    responseDeserialize: (value: Buffer) => TransactionsResponse.decode(value),
  },
  /** Get events with metadata from transactions, supporting the same filtering as GetTransactions. */
  getEvents: {
    path: "/aptos.indexer.v1.RawData/GetEvents",
    requestStream: false,
    responseStream: true,
    requestSerialize: (value: GetEventsRequest) => Buffer.from(GetEventsRequest.encode(value).finish()),
    requestDeserialize: (value: Buffer) => GetEventsRequest.decode(value),
    responseSerialize: (value: EventsResponse) => Buffer.from(EventsResponse.encode(value).finish()),
    responseDeserialize: (value: Buffer) => EventsResponse.decode(value),
  },
} as const;

export interface RawDataServer extends UntypedServiceImplementation {
  /** Get transactions batch without any filtering from starting version and end if transaction count is present. */
  getTransactions: handleServerStreamingCall<GetTransactionsRequest, TransactionsResponse>;
  /** Get events with metadata from transactions, supporting the same filtering as GetTransactions. */
  getEvents: handleServerStreamingCall<GetEventsRequest, EventsResponse>;
}

export interface RawDataClient extends Client {
  /** Get transactions batch without any filtering from starting version and end if transaction count is present. */
  getTransactions(
    request: GetTransactionsRequest,
    options?: Partial<CallOptions>,
  ): ClientReadableStream<TransactionsResponse>;
  getTransactions(
    request: GetTransactionsRequest,
    metadata?: Metadata,
    options?: Partial<CallOptions>,
  ): ClientReadableStream<TransactionsResponse>;
  /** Get events with metadata from transactions, supporting the same filtering as GetTransactions. */
  getEvents(request: GetEventsRequest, options?: Partial<CallOptions>): ClientReadableStream<EventsResponse>;
  getEvents(
    request: GetEventsRequest,
    metadata?: Metadata,
    options?: Partial<CallOptions>,
  ): ClientReadableStream<EventsResponse>;
}

export const RawDataClient = makeGenericClientConstructor(RawDataService, "aptos.indexer.v1.RawData") as unknown as {
  new (address: string, credentials: ChannelCredentials, options?: Partial<ClientOptions>): RawDataClient;
  service: typeof RawDataService;
  serviceName: string;
};

function bytesFromBase64(b64: string): Uint8Array {
  if ((globalThis as any).Buffer) {
    return Uint8Array.from(globalThis.Buffer.from(b64, "base64"));
  } else {
    const bin = globalThis.atob(b64);
    const arr = new Uint8Array(bin.length);
    for (let i = 0; i < bin.length; ++i) {
      arr[i] = bin.charCodeAt(i);
    }
    return arr;
  }
}

function base64FromBytes(arr: Uint8Array): string {
  if ((globalThis as any).Buffer) {
    return globalThis.Buffer.from(arr).toString("base64");
  } else {
    const bin: string[] = [];
    arr.forEach((byte) => {
      bin.push(globalThis.String.fromCharCode(byte));
    });
    return globalThis.btoa(bin.join(""));
  }
}

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
