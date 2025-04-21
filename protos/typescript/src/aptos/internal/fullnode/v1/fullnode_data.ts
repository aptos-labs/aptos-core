/* eslint-disable */
import {
  ChannelCredentials,
  Client,
  ClientReadableStream,
  handleServerStreamingCall,
  makeGenericClientConstructor,
  Metadata,
} from "@grpc/grpc-js";
import type {
  CallOptions,
  ClientOptions,
  ClientUnaryCall,
  handleUnaryCall,
  ServiceError,
  UntypedServiceImplementation,
} from "@grpc/grpc-js";
import Long from "long";
import _m0 from "protobufjs/minimal";
import { FullnodeInfo } from "../../../indexer/v1/grpc";
import { Transaction } from "../../../transaction/v1/transaction";

export interface TransactionsOutput {
  transactions?: Transaction[] | undefined;
}

export interface StreamStatus {
  type?:
    | StreamStatus_StatusType
    | undefined;
  /** Required. Start version of current batch/stream, inclusive. */
  startVersion?:
    | bigint
    | undefined;
  /** End version of current *batch*, inclusive. */
  endVersion?: bigint | undefined;
}

export enum StreamStatus_StatusType {
  STATUS_TYPE_UNSPECIFIED = 0,
  /** STATUS_TYPE_INIT - Signal for the start of the stream. */
  STATUS_TYPE_INIT = 1,
  /** STATUS_TYPE_BATCH_END - Signal for the end of the batch. */
  STATUS_TYPE_BATCH_END = 2,
  UNRECOGNIZED = -1,
}

export function streamStatus_StatusTypeFromJSON(object: any): StreamStatus_StatusType {
  switch (object) {
    case 0:
    case "STATUS_TYPE_UNSPECIFIED":
      return StreamStatus_StatusType.STATUS_TYPE_UNSPECIFIED;
    case 1:
    case "STATUS_TYPE_INIT":
      return StreamStatus_StatusType.STATUS_TYPE_INIT;
    case 2:
    case "STATUS_TYPE_BATCH_END":
      return StreamStatus_StatusType.STATUS_TYPE_BATCH_END;
    case -1:
    case "UNRECOGNIZED":
    default:
      return StreamStatus_StatusType.UNRECOGNIZED;
  }
}

export function streamStatus_StatusTypeToJSON(object: StreamStatus_StatusType): string {
  switch (object) {
    case StreamStatus_StatusType.STATUS_TYPE_UNSPECIFIED:
      return "STATUS_TYPE_UNSPECIFIED";
    case StreamStatus_StatusType.STATUS_TYPE_INIT:
      return "STATUS_TYPE_INIT";
    case StreamStatus_StatusType.STATUS_TYPE_BATCH_END:
      return "STATUS_TYPE_BATCH_END";
    case StreamStatus_StatusType.UNRECOGNIZED:
    default:
      return "UNRECOGNIZED";
  }
}

export interface GetTransactionsFromNodeRequest {
  /**
   * Required; start version of current stream.
   * If not set will panic somewhere
   */
  startingVersion?:
    | bigint
    | undefined;
  /**
   * Optional; number of transactions to return in current stream.
   * If not set, response streams infinitely.
   */
  transactionsCount?: bigint | undefined;
}

export interface TransactionsFromNodeResponse {
  status?: StreamStatus | undefined;
  data?:
    | TransactionsOutput
    | undefined;
  /** Making sure that all the responses include a chain id */
  chainId?: number | undefined;
}

export interface PingFullnodeRequest {
}

export interface PingFullnodeResponse {
  info?: FullnodeInfo | undefined;
}

function createBaseTransactionsOutput(): TransactionsOutput {
  return { transactions: [] };
}

export const TransactionsOutput = {
  encode(message: TransactionsOutput, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.transactions !== undefined && message.transactions.length !== 0) {
      for (const v of message.transactions) {
        Transaction.encode(v!, writer.uint32(10).fork()).ldelim();
      }
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): TransactionsOutput {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseTransactionsOutput();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.transactions!.push(Transaction.decode(reader, reader.uint32()));
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
  // Transform<TransactionsOutput, Uint8Array>
  async *encodeTransform(
    source:
      | AsyncIterable<TransactionsOutput | TransactionsOutput[]>
      | Iterable<TransactionsOutput | TransactionsOutput[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [TransactionsOutput.encode(p).finish()];
        }
      } else {
        yield* [TransactionsOutput.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, TransactionsOutput>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<TransactionsOutput> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [TransactionsOutput.decode(p)];
        }
      } else {
        yield* [TransactionsOutput.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): TransactionsOutput {
    return {
      transactions: globalThis.Array.isArray(object?.transactions)
        ? object.transactions.map((e: any) => Transaction.fromJSON(e))
        : [],
    };
  },

  toJSON(message: TransactionsOutput): unknown {
    const obj: any = {};
    if (message.transactions?.length) {
      obj.transactions = message.transactions.map((e) => Transaction.toJSON(e));
    }
    return obj;
  },

  create(base?: DeepPartial<TransactionsOutput>): TransactionsOutput {
    return TransactionsOutput.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<TransactionsOutput>): TransactionsOutput {
    const message = createBaseTransactionsOutput();
    message.transactions = object.transactions?.map((e) => Transaction.fromPartial(e)) || [];
    return message;
  },
};

function createBaseStreamStatus(): StreamStatus {
  return { type: 0, startVersion: BigInt("0"), endVersion: undefined };
}

export const StreamStatus = {
  encode(message: StreamStatus, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.type !== undefined && message.type !== 0) {
      writer.uint32(8).int32(message.type);
    }
    if (message.startVersion !== undefined && message.startVersion !== BigInt("0")) {
      if (BigInt.asUintN(64, message.startVersion) !== message.startVersion) {
        throw new globalThis.Error("value provided for field message.startVersion of type uint64 too large");
      }
      writer.uint32(16).uint64(message.startVersion.toString());
    }
    if (message.endVersion !== undefined) {
      if (BigInt.asUintN(64, message.endVersion) !== message.endVersion) {
        throw new globalThis.Error("value provided for field message.endVersion of type uint64 too large");
      }
      writer.uint32(24).uint64(message.endVersion.toString());
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): StreamStatus {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseStreamStatus();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 8) {
            break;
          }

          message.type = reader.int32() as any;
          continue;
        case 2:
          if (tag !== 16) {
            break;
          }

          message.startVersion = longToBigint(reader.uint64() as Long);
          continue;
        case 3:
          if (tag !== 24) {
            break;
          }

          message.endVersion = longToBigint(reader.uint64() as Long);
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
  // Transform<StreamStatus, Uint8Array>
  async *encodeTransform(
    source: AsyncIterable<StreamStatus | StreamStatus[]> | Iterable<StreamStatus | StreamStatus[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [StreamStatus.encode(p).finish()];
        }
      } else {
        yield* [StreamStatus.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, StreamStatus>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<StreamStatus> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [StreamStatus.decode(p)];
        }
      } else {
        yield* [StreamStatus.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): StreamStatus {
    return {
      type: isSet(object.type) ? streamStatus_StatusTypeFromJSON(object.type) : 0,
      startVersion: isSet(object.startVersion) ? BigInt(object.startVersion) : BigInt("0"),
      endVersion: isSet(object.endVersion) ? BigInt(object.endVersion) : undefined,
    };
  },

  toJSON(message: StreamStatus): unknown {
    const obj: any = {};
    if (message.type !== undefined && message.type !== 0) {
      obj.type = streamStatus_StatusTypeToJSON(message.type);
    }
    if (message.startVersion !== undefined && message.startVersion !== BigInt("0")) {
      obj.startVersion = message.startVersion.toString();
    }
    if (message.endVersion !== undefined) {
      obj.endVersion = message.endVersion.toString();
    }
    return obj;
  },

  create(base?: DeepPartial<StreamStatus>): StreamStatus {
    return StreamStatus.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<StreamStatus>): StreamStatus {
    const message = createBaseStreamStatus();
    message.type = object.type ?? 0;
    message.startVersion = object.startVersion ?? BigInt("0");
    message.endVersion = object.endVersion ?? undefined;
    return message;
  },
};

function createBaseGetTransactionsFromNodeRequest(): GetTransactionsFromNodeRequest {
  return { startingVersion: undefined, transactionsCount: undefined };
}

export const GetTransactionsFromNodeRequest = {
  encode(message: GetTransactionsFromNodeRequest, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
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
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): GetTransactionsFromNodeRequest {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseGetTransactionsFromNodeRequest();
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
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  // encodeTransform encodes a source of message objects.
  // Transform<GetTransactionsFromNodeRequest, Uint8Array>
  async *encodeTransform(
    source:
      | AsyncIterable<GetTransactionsFromNodeRequest | GetTransactionsFromNodeRequest[]>
      | Iterable<GetTransactionsFromNodeRequest | GetTransactionsFromNodeRequest[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [GetTransactionsFromNodeRequest.encode(p).finish()];
        }
      } else {
        yield* [GetTransactionsFromNodeRequest.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, GetTransactionsFromNodeRequest>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<GetTransactionsFromNodeRequest> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [GetTransactionsFromNodeRequest.decode(p)];
        }
      } else {
        yield* [GetTransactionsFromNodeRequest.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): GetTransactionsFromNodeRequest {
    return {
      startingVersion: isSet(object.startingVersion) ? BigInt(object.startingVersion) : undefined,
      transactionsCount: isSet(object.transactionsCount) ? BigInt(object.transactionsCount) : undefined,
    };
  },

  toJSON(message: GetTransactionsFromNodeRequest): unknown {
    const obj: any = {};
    if (message.startingVersion !== undefined) {
      obj.startingVersion = message.startingVersion.toString();
    }
    if (message.transactionsCount !== undefined) {
      obj.transactionsCount = message.transactionsCount.toString();
    }
    return obj;
  },

  create(base?: DeepPartial<GetTransactionsFromNodeRequest>): GetTransactionsFromNodeRequest {
    return GetTransactionsFromNodeRequest.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<GetTransactionsFromNodeRequest>): GetTransactionsFromNodeRequest {
    const message = createBaseGetTransactionsFromNodeRequest();
    message.startingVersion = object.startingVersion ?? undefined;
    message.transactionsCount = object.transactionsCount ?? undefined;
    return message;
  },
};

function createBaseTransactionsFromNodeResponse(): TransactionsFromNodeResponse {
  return { status: undefined, data: undefined, chainId: 0 };
}

export const TransactionsFromNodeResponse = {
  encode(message: TransactionsFromNodeResponse, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.status !== undefined) {
      StreamStatus.encode(message.status, writer.uint32(10).fork()).ldelim();
    }
    if (message.data !== undefined) {
      TransactionsOutput.encode(message.data, writer.uint32(18).fork()).ldelim();
    }
    if (message.chainId !== undefined && message.chainId !== 0) {
      writer.uint32(24).uint32(message.chainId);
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): TransactionsFromNodeResponse {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseTransactionsFromNodeResponse();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.status = StreamStatus.decode(reader, reader.uint32());
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.data = TransactionsOutput.decode(reader, reader.uint32());
          continue;
        case 3:
          if (tag !== 24) {
            break;
          }

          message.chainId = reader.uint32();
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
  // Transform<TransactionsFromNodeResponse, Uint8Array>
  async *encodeTransform(
    source:
      | AsyncIterable<TransactionsFromNodeResponse | TransactionsFromNodeResponse[]>
      | Iterable<TransactionsFromNodeResponse | TransactionsFromNodeResponse[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [TransactionsFromNodeResponse.encode(p).finish()];
        }
      } else {
        yield* [TransactionsFromNodeResponse.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, TransactionsFromNodeResponse>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<TransactionsFromNodeResponse> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [TransactionsFromNodeResponse.decode(p)];
        }
      } else {
        yield* [TransactionsFromNodeResponse.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): TransactionsFromNodeResponse {
    return {
      status: isSet(object.status) ? StreamStatus.fromJSON(object.status) : undefined,
      data: isSet(object.data) ? TransactionsOutput.fromJSON(object.data) : undefined,
      chainId: isSet(object.chainId) ? globalThis.Number(object.chainId) : 0,
    };
  },

  toJSON(message: TransactionsFromNodeResponse): unknown {
    const obj: any = {};
    if (message.status !== undefined) {
      obj.status = StreamStatus.toJSON(message.status);
    }
    if (message.data !== undefined) {
      obj.data = TransactionsOutput.toJSON(message.data);
    }
    if (message.chainId !== undefined && message.chainId !== 0) {
      obj.chainId = Math.round(message.chainId);
    }
    return obj;
  },

  create(base?: DeepPartial<TransactionsFromNodeResponse>): TransactionsFromNodeResponse {
    return TransactionsFromNodeResponse.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<TransactionsFromNodeResponse>): TransactionsFromNodeResponse {
    const message = createBaseTransactionsFromNodeResponse();
    message.status = (object.status !== undefined && object.status !== null)
      ? StreamStatus.fromPartial(object.status)
      : undefined;
    message.data = (object.data !== undefined && object.data !== null)
      ? TransactionsOutput.fromPartial(object.data)
      : undefined;
    message.chainId = object.chainId ?? 0;
    return message;
  },
};

function createBasePingFullnodeRequest(): PingFullnodeRequest {
  return {};
}

export const PingFullnodeRequest = {
  encode(_: PingFullnodeRequest, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): PingFullnodeRequest {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBasePingFullnodeRequest();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  // encodeTransform encodes a source of message objects.
  // Transform<PingFullnodeRequest, Uint8Array>
  async *encodeTransform(
    source:
      | AsyncIterable<PingFullnodeRequest | PingFullnodeRequest[]>
      | Iterable<PingFullnodeRequest | PingFullnodeRequest[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [PingFullnodeRequest.encode(p).finish()];
        }
      } else {
        yield* [PingFullnodeRequest.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, PingFullnodeRequest>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<PingFullnodeRequest> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [PingFullnodeRequest.decode(p)];
        }
      } else {
        yield* [PingFullnodeRequest.decode(pkt as any)];
      }
    }
  },

  fromJSON(_: any): PingFullnodeRequest {
    return {};
  },

  toJSON(_: PingFullnodeRequest): unknown {
    const obj: any = {};
    return obj;
  },

  create(base?: DeepPartial<PingFullnodeRequest>): PingFullnodeRequest {
    return PingFullnodeRequest.fromPartial(base ?? {});
  },
  fromPartial(_: DeepPartial<PingFullnodeRequest>): PingFullnodeRequest {
    const message = createBasePingFullnodeRequest();
    return message;
  },
};

function createBasePingFullnodeResponse(): PingFullnodeResponse {
  return { info: undefined };
}

export const PingFullnodeResponse = {
  encode(message: PingFullnodeResponse, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.info !== undefined) {
      FullnodeInfo.encode(message.info, writer.uint32(10).fork()).ldelim();
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): PingFullnodeResponse {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBasePingFullnodeResponse();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.info = FullnodeInfo.decode(reader, reader.uint32());
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
  // Transform<PingFullnodeResponse, Uint8Array>
  async *encodeTransform(
    source:
      | AsyncIterable<PingFullnodeResponse | PingFullnodeResponse[]>
      | Iterable<PingFullnodeResponse | PingFullnodeResponse[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [PingFullnodeResponse.encode(p).finish()];
        }
      } else {
        yield* [PingFullnodeResponse.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, PingFullnodeResponse>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<PingFullnodeResponse> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [PingFullnodeResponse.decode(p)];
        }
      } else {
        yield* [PingFullnodeResponse.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): PingFullnodeResponse {
    return { info: isSet(object.info) ? FullnodeInfo.fromJSON(object.info) : undefined };
  },

  toJSON(message: PingFullnodeResponse): unknown {
    const obj: any = {};
    if (message.info !== undefined) {
      obj.info = FullnodeInfo.toJSON(message.info);
    }
    return obj;
  },

  create(base?: DeepPartial<PingFullnodeResponse>): PingFullnodeResponse {
    return PingFullnodeResponse.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<PingFullnodeResponse>): PingFullnodeResponse {
    const message = createBasePingFullnodeResponse();
    message.info = (object.info !== undefined && object.info !== null)
      ? FullnodeInfo.fromPartial(object.info)
      : undefined;
    return message;
  },
};

export type FullnodeDataService = typeof FullnodeDataService;
export const FullnodeDataService = {
  ping: {
    path: "/aptos.internal.fullnode.v1.FullnodeData/Ping",
    requestStream: false,
    responseStream: false,
    requestSerialize: (value: PingFullnodeRequest) => Buffer.from(PingFullnodeRequest.encode(value).finish()),
    requestDeserialize: (value: Buffer) => PingFullnodeRequest.decode(value),
    responseSerialize: (value: PingFullnodeResponse) => Buffer.from(PingFullnodeResponse.encode(value).finish()),
    responseDeserialize: (value: Buffer) => PingFullnodeResponse.decode(value),
  },
  getTransactionsFromNode: {
    path: "/aptos.internal.fullnode.v1.FullnodeData/GetTransactionsFromNode",
    requestStream: false,
    responseStream: true,
    requestSerialize: (value: GetTransactionsFromNodeRequest) =>
      Buffer.from(GetTransactionsFromNodeRequest.encode(value).finish()),
    requestDeserialize: (value: Buffer) => GetTransactionsFromNodeRequest.decode(value),
    responseSerialize: (value: TransactionsFromNodeResponse) =>
      Buffer.from(TransactionsFromNodeResponse.encode(value).finish()),
    responseDeserialize: (value: Buffer) => TransactionsFromNodeResponse.decode(value),
  },
} as const;

export interface FullnodeDataServer extends UntypedServiceImplementation {
  ping: handleUnaryCall<PingFullnodeRequest, PingFullnodeResponse>;
  getTransactionsFromNode: handleServerStreamingCall<GetTransactionsFromNodeRequest, TransactionsFromNodeResponse>;
}

export interface FullnodeDataClient extends Client {
  ping(
    request: PingFullnodeRequest,
    callback: (error: ServiceError | null, response: PingFullnodeResponse) => void,
  ): ClientUnaryCall;
  ping(
    request: PingFullnodeRequest,
    metadata: Metadata,
    callback: (error: ServiceError | null, response: PingFullnodeResponse) => void,
  ): ClientUnaryCall;
  ping(
    request: PingFullnodeRequest,
    metadata: Metadata,
    options: Partial<CallOptions>,
    callback: (error: ServiceError | null, response: PingFullnodeResponse) => void,
  ): ClientUnaryCall;
  getTransactionsFromNode(
    request: GetTransactionsFromNodeRequest,
    options?: Partial<CallOptions>,
  ): ClientReadableStream<TransactionsFromNodeResponse>;
  getTransactionsFromNode(
    request: GetTransactionsFromNodeRequest,
    metadata?: Metadata,
    options?: Partial<CallOptions>,
  ): ClientReadableStream<TransactionsFromNodeResponse>;
}

export const FullnodeDataClient = makeGenericClientConstructor(
  FullnodeDataService,
  "aptos.internal.fullnode.v1.FullnodeData",
) as unknown as {
  new (address: string, credentials: ChannelCredentials, options?: Partial<ClientOptions>): FullnodeDataClient;
  service: typeof FullnodeDataService;
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
