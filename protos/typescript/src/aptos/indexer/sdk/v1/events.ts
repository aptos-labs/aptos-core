/* eslint-disable */
import {
  ChannelCredentials,
  Client,
  ClientDuplexStream,
  handleBidiStreamingCall,
  makeGenericClientConstructor,
  Metadata,
} from "@grpc/grpc-js";
import type { CallOptions, ClientOptions, UntypedServiceImplementation } from "@grpc/grpc-js";
import Long from "long";
import _m0 from "protobufjs/minimal";
import { Event } from "../../../transaction/v1/transaction";
import { Timestamp } from "../../../util/timestamp/timestamp";

export interface SdkEventsStepRequest {
  transactionContext?: TransactionContext | undefined;
}

/** todo create a way to surface errors. errors should be recoverable or not. */
export interface SdkEventsStepResponse {
  startVersion?: bigint | undefined;
  endVersion?: bigint | undefined;
}

export interface TransactionContext {
  events?: Event[] | undefined;
  startVersion?: bigint | undefined;
  endVersion?: bigint | undefined;
  startTransactionTimestamp?: Timestamp | undefined;
  endTransactionTimestamp?: Timestamp | undefined;
  totalSizeInBytes?: bigint | undefined;
}

function createBaseSdkEventsStepRequest(): SdkEventsStepRequest {
  return { transactionContext: undefined };
}

export const SdkEventsStepRequest = {
  encode(message: SdkEventsStepRequest, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.transactionContext !== undefined) {
      TransactionContext.encode(message.transactionContext, writer.uint32(10).fork()).ldelim();
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): SdkEventsStepRequest {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseSdkEventsStepRequest();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.transactionContext = TransactionContext.decode(reader, reader.uint32());
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
  // Transform<SdkEventsStepRequest, Uint8Array>
  async *encodeTransform(
    source:
      | AsyncIterable<SdkEventsStepRequest | SdkEventsStepRequest[]>
      | Iterable<SdkEventsStepRequest | SdkEventsStepRequest[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [SdkEventsStepRequest.encode(p).finish()];
        }
      } else {
        yield* [SdkEventsStepRequest.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, SdkEventsStepRequest>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<SdkEventsStepRequest> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [SdkEventsStepRequest.decode(p)];
        }
      } else {
        yield* [SdkEventsStepRequest.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): SdkEventsStepRequest {
    return {
      transactionContext: isSet(object.transactionContext)
        ? TransactionContext.fromJSON(object.transactionContext)
        : undefined,
    };
  },

  toJSON(message: SdkEventsStepRequest): unknown {
    const obj: any = {};
    if (message.transactionContext !== undefined) {
      obj.transactionContext = TransactionContext.toJSON(message.transactionContext);
    }
    return obj;
  },

  create(base?: DeepPartial<SdkEventsStepRequest>): SdkEventsStepRequest {
    return SdkEventsStepRequest.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<SdkEventsStepRequest>): SdkEventsStepRequest {
    const message = createBaseSdkEventsStepRequest();
    message.transactionContext = (object.transactionContext !== undefined && object.transactionContext !== null)
      ? TransactionContext.fromPartial(object.transactionContext)
      : undefined;
    return message;
  },
};

function createBaseSdkEventsStepResponse(): SdkEventsStepResponse {
  return { startVersion: BigInt("0"), endVersion: BigInt("0") };
}

export const SdkEventsStepResponse = {
  encode(message: SdkEventsStepResponse, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.startVersion !== undefined && message.startVersion !== BigInt("0")) {
      if (BigInt.asUintN(64, message.startVersion) !== message.startVersion) {
        throw new globalThis.Error("value provided for field message.startVersion of type uint64 too large");
      }
      writer.uint32(8).uint64(message.startVersion.toString());
    }
    if (message.endVersion !== undefined && message.endVersion !== BigInt("0")) {
      if (BigInt.asUintN(64, message.endVersion) !== message.endVersion) {
        throw new globalThis.Error("value provided for field message.endVersion of type uint64 too large");
      }
      writer.uint32(16).uint64(message.endVersion.toString());
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): SdkEventsStepResponse {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseSdkEventsStepResponse();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 8) {
            break;
          }

          message.startVersion = longToBigint(reader.uint64() as Long);
          continue;
        case 2:
          if (tag !== 16) {
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
  // Transform<SdkEventsStepResponse, Uint8Array>
  async *encodeTransform(
    source:
      | AsyncIterable<SdkEventsStepResponse | SdkEventsStepResponse[]>
      | Iterable<SdkEventsStepResponse | SdkEventsStepResponse[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [SdkEventsStepResponse.encode(p).finish()];
        }
      } else {
        yield* [SdkEventsStepResponse.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, SdkEventsStepResponse>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<SdkEventsStepResponse> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [SdkEventsStepResponse.decode(p)];
        }
      } else {
        yield* [SdkEventsStepResponse.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): SdkEventsStepResponse {
    return {
      startVersion: isSet(object.startVersion) ? BigInt(object.startVersion) : BigInt("0"),
      endVersion: isSet(object.endVersion) ? BigInt(object.endVersion) : BigInt("0"),
    };
  },

  toJSON(message: SdkEventsStepResponse): unknown {
    const obj: any = {};
    if (message.startVersion !== undefined && message.startVersion !== BigInt("0")) {
      obj.startVersion = message.startVersion.toString();
    }
    if (message.endVersion !== undefined && message.endVersion !== BigInt("0")) {
      obj.endVersion = message.endVersion.toString();
    }
    return obj;
  },

  create(base?: DeepPartial<SdkEventsStepResponse>): SdkEventsStepResponse {
    return SdkEventsStepResponse.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<SdkEventsStepResponse>): SdkEventsStepResponse {
    const message = createBaseSdkEventsStepResponse();
    message.startVersion = object.startVersion ?? BigInt("0");
    message.endVersion = object.endVersion ?? BigInt("0");
    return message;
  },
};

function createBaseTransactionContext(): TransactionContext {
  return {
    events: [],
    startVersion: BigInt("0"),
    endVersion: BigInt("0"),
    startTransactionTimestamp: undefined,
    endTransactionTimestamp: undefined,
    totalSizeInBytes: BigInt("0"),
  };
}

export const TransactionContext = {
  encode(message: TransactionContext, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.events !== undefined && message.events.length !== 0) {
      for (const v of message.events) {
        Event.encode(v!, writer.uint32(10).fork()).ldelim();
      }
    }
    if (message.startVersion !== undefined && message.startVersion !== BigInt("0")) {
      if (BigInt.asUintN(64, message.startVersion) !== message.startVersion) {
        throw new globalThis.Error("value provided for field message.startVersion of type uint64 too large");
      }
      writer.uint32(16).uint64(message.startVersion.toString());
    }
    if (message.endVersion !== undefined && message.endVersion !== BigInt("0")) {
      if (BigInt.asUintN(64, message.endVersion) !== message.endVersion) {
        throw new globalThis.Error("value provided for field message.endVersion of type uint64 too large");
      }
      writer.uint32(24).uint64(message.endVersion.toString());
    }
    if (message.startTransactionTimestamp !== undefined) {
      Timestamp.encode(message.startTransactionTimestamp, writer.uint32(34).fork()).ldelim();
    }
    if (message.endTransactionTimestamp !== undefined) {
      Timestamp.encode(message.endTransactionTimestamp, writer.uint32(42).fork()).ldelim();
    }
    if (message.totalSizeInBytes !== undefined && message.totalSizeInBytes !== BigInt("0")) {
      if (BigInt.asUintN(64, message.totalSizeInBytes) !== message.totalSizeInBytes) {
        throw new globalThis.Error("value provided for field message.totalSizeInBytes of type uint64 too large");
      }
      writer.uint32(48).uint64(message.totalSizeInBytes.toString());
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): TransactionContext {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseTransactionContext();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.events!.push(Event.decode(reader, reader.uint32()));
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
        case 4:
          if (tag !== 34) {
            break;
          }

          message.startTransactionTimestamp = Timestamp.decode(reader, reader.uint32());
          continue;
        case 5:
          if (tag !== 42) {
            break;
          }

          message.endTransactionTimestamp = Timestamp.decode(reader, reader.uint32());
          continue;
        case 6:
          if (tag !== 48) {
            break;
          }

          message.totalSizeInBytes = longToBigint(reader.uint64() as Long);
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
  // Transform<TransactionContext, Uint8Array>
  async *encodeTransform(
    source:
      | AsyncIterable<TransactionContext | TransactionContext[]>
      | Iterable<TransactionContext | TransactionContext[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [TransactionContext.encode(p).finish()];
        }
      } else {
        yield* [TransactionContext.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, TransactionContext>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<TransactionContext> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [TransactionContext.decode(p)];
        }
      } else {
        yield* [TransactionContext.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): TransactionContext {
    return {
      events: globalThis.Array.isArray(object?.events) ? object.events.map((e: any) => Event.fromJSON(e)) : [],
      startVersion: isSet(object.startVersion) ? BigInt(object.startVersion) : BigInt("0"),
      endVersion: isSet(object.endVersion) ? BigInt(object.endVersion) : BigInt("0"),
      startTransactionTimestamp: isSet(object.startTransactionTimestamp)
        ? Timestamp.fromJSON(object.startTransactionTimestamp)
        : undefined,
      endTransactionTimestamp: isSet(object.endTransactionTimestamp)
        ? Timestamp.fromJSON(object.endTransactionTimestamp)
        : undefined,
      totalSizeInBytes: isSet(object.totalSizeInBytes) ? BigInt(object.totalSizeInBytes) : BigInt("0"),
    };
  },

  toJSON(message: TransactionContext): unknown {
    const obj: any = {};
    if (message.events?.length) {
      obj.events = message.events.map((e) => Event.toJSON(e));
    }
    if (message.startVersion !== undefined && message.startVersion !== BigInt("0")) {
      obj.startVersion = message.startVersion.toString();
    }
    if (message.endVersion !== undefined && message.endVersion !== BigInt("0")) {
      obj.endVersion = message.endVersion.toString();
    }
    if (message.startTransactionTimestamp !== undefined) {
      obj.startTransactionTimestamp = Timestamp.toJSON(message.startTransactionTimestamp);
    }
    if (message.endTransactionTimestamp !== undefined) {
      obj.endTransactionTimestamp = Timestamp.toJSON(message.endTransactionTimestamp);
    }
    if (message.totalSizeInBytes !== undefined && message.totalSizeInBytes !== BigInt("0")) {
      obj.totalSizeInBytes = message.totalSizeInBytes.toString();
    }
    return obj;
  },

  create(base?: DeepPartial<TransactionContext>): TransactionContext {
    return TransactionContext.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<TransactionContext>): TransactionContext {
    const message = createBaseTransactionContext();
    message.events = object.events?.map((e) => Event.fromPartial(e)) || [];
    message.startVersion = object.startVersion ?? BigInt("0");
    message.endVersion = object.endVersion ?? BigInt("0");
    message.startTransactionTimestamp =
      (object.startTransactionTimestamp !== undefined && object.startTransactionTimestamp !== null)
        ? Timestamp.fromPartial(object.startTransactionTimestamp)
        : undefined;
    message.endTransactionTimestamp =
      (object.endTransactionTimestamp !== undefined && object.endTransactionTimestamp !== null)
        ? Timestamp.fromPartial(object.endTransactionTimestamp)
        : undefined;
    message.totalSizeInBytes = object.totalSizeInBytes ?? BigInt("0");
    return message;
  },
};

/**
 * The SDK is the client in this model. It sends transactions to the lambda step over
 * the stream (SdkEventsStepRequest) and receives a response (SdkEventsStepResponse)
 * from the lambda step.
 */
export type SdkEventsStepServiceService = typeof SdkEventsStepServiceService;
export const SdkEventsStepServiceService = {
  bidirectionalStream: {
    path: "/aptos.indexer.sdk.v1.SdkEventsStepService/BidirectionalStream",
    requestStream: true,
    responseStream: true,
    requestSerialize: (value: SdkEventsStepRequest) => Buffer.from(SdkEventsStepRequest.encode(value).finish()),
    requestDeserialize: (value: Buffer) => SdkEventsStepRequest.decode(value),
    responseSerialize: (value: SdkEventsStepResponse) => Buffer.from(SdkEventsStepResponse.encode(value).finish()),
    responseDeserialize: (value: Buffer) => SdkEventsStepResponse.decode(value),
  },
} as const;

export interface SdkEventsStepServiceServer extends UntypedServiceImplementation {
  bidirectionalStream: handleBidiStreamingCall<SdkEventsStepRequest, SdkEventsStepResponse>;
}

export interface SdkEventsStepServiceClient extends Client {
  bidirectionalStream(): ClientDuplexStream<SdkEventsStepRequest, SdkEventsStepResponse>;
  bidirectionalStream(options: Partial<CallOptions>): ClientDuplexStream<SdkEventsStepRequest, SdkEventsStepResponse>;
  bidirectionalStream(
    metadata: Metadata,
    options?: Partial<CallOptions>,
  ): ClientDuplexStream<SdkEventsStepRequest, SdkEventsStepResponse>;
}

export const SdkEventsStepServiceClient = makeGenericClientConstructor(
  SdkEventsStepServiceService,
  "aptos.indexer.sdk.v1.SdkEventsStepService",
) as unknown as {
  new (address: string, credentials: ChannelCredentials, options?: Partial<ClientOptions>): SdkEventsStepServiceClient;
  service: typeof SdkEventsStepServiceService;
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
