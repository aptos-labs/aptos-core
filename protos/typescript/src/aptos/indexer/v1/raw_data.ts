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
import { Transaction } from "../../transaction/v1/transaction";
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

export type RawDataService = typeof RawDataService;
export const RawDataService = {
  /** Get transactions batch without any filtering from starting version and end if transaction count is present. */
  getTransactions: {
    path: "/velor.indexer.v1.RawData/GetTransactions",
    requestStream: false,
    responseStream: true,
    requestSerialize: (value: GetTransactionsRequest) => Buffer.from(GetTransactionsRequest.encode(value).finish()),
    requestDeserialize: (value: Buffer) => GetTransactionsRequest.decode(value),
    responseSerialize: (value: TransactionsResponse) => Buffer.from(TransactionsResponse.encode(value).finish()),
    responseDeserialize: (value: Buffer) => TransactionsResponse.decode(value),
  },
} as const;

export interface RawDataServer extends UntypedServiceImplementation {
  /** Get transactions batch without any filtering from starting version and end if transaction count is present. */
  getTransactions: handleServerStreamingCall<GetTransactionsRequest, TransactionsResponse>;
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
}

export const RawDataClient = makeGenericClientConstructor(RawDataService, "velor.indexer.v1.RawData") as unknown as {
  new (address: string, credentials: ChannelCredentials, options?: Partial<ClientOptions>): RawDataClient;
  service: typeof RawDataService;
  serviceName: string;
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
