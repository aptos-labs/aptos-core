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
import { EventKey, MoveType } from "../../transaction/v1/transaction";
import { Timestamp } from "../../util/timestamp/timestamp";

export interface Event {
  key?: EventKey | undefined;
  sequenceNumber?: bigint | undefined;
  type?: MoveType | undefined;
  typeStr?: string | undefined;
  data?: string | undefined;
  transactionVersion?: bigint | undefined;
  transactionTimestamp?: Timestamp | undefined;
}

/** This is for storage only. */
export interface EventsInStorage {
  /** Required; event data. */
  transactions?:
    | Event[]
    | undefined;
  /** Required; chain id. (?) */
  startingVersion?: bigint | undefined;
}

export interface GetEventsRequest {
  /** Required; start version of current stream. */
  startingVersion?:
    | bigint
    | undefined;
  /**
   * Optional; number of events to return in current stream.
   * If not present, return an infinite stream of events.
   */
  eventsCount?:
    | bigint
    | undefined;
  /**
   * Optional; number of events in each `EventsResponse` for current stream.
   * If not present, default to 1000. If larger than 1000, request will be rejected.
   */
  batchSize?: bigint | undefined;
}

/** EventsResponse is a batch of transactions. */
export interface EventsResponse {
  /** Required; transactions data. */
  events?:
    | Event[]
    | undefined;
  /** Required; chain id. */
  chainId?: bigint | undefined;
}

function createBaseEvent(): Event {
  return {
    key: undefined,
    sequenceNumber: BigInt("0"),
    type: undefined,
    typeStr: "",
    data: "",
    transactionVersion: BigInt("0"),
    transactionTimestamp: undefined,
  };
}

export const Event = {
  encode(message: Event, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.key !== undefined) {
      EventKey.encode(message.key, writer.uint32(10).fork()).ldelim();
    }
    if (message.sequenceNumber !== undefined && message.sequenceNumber !== BigInt("0")) {
      if (BigInt.asUintN(64, message.sequenceNumber) !== message.sequenceNumber) {
        throw new globalThis.Error("value provided for field message.sequenceNumber of type uint64 too large");
      }
      writer.uint32(16).uint64(message.sequenceNumber.toString());
    }
    if (message.type !== undefined) {
      MoveType.encode(message.type, writer.uint32(26).fork()).ldelim();
    }
    if (message.typeStr !== undefined && message.typeStr !== "") {
      writer.uint32(42).string(message.typeStr);
    }
    if (message.data !== undefined && message.data !== "") {
      writer.uint32(34).string(message.data);
    }
    if (message.transactionVersion !== undefined && message.transactionVersion !== BigInt("0")) {
      if (BigInt.asUintN(64, message.transactionVersion) !== message.transactionVersion) {
        throw new globalThis.Error("value provided for field message.transactionVersion of type uint64 too large");
      }
      writer.uint32(48).uint64(message.transactionVersion.toString());
    }
    if (message.transactionTimestamp !== undefined) {
      Timestamp.encode(message.transactionTimestamp, writer.uint32(58).fork()).ldelim();
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): Event {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseEvent();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.key = EventKey.decode(reader, reader.uint32());
          continue;
        case 2:
          if (tag !== 16) {
            break;
          }

          message.sequenceNumber = longToBigint(reader.uint64() as Long);
          continue;
        case 3:
          if (tag !== 26) {
            break;
          }

          message.type = MoveType.decode(reader, reader.uint32());
          continue;
        case 5:
          if (tag !== 42) {
            break;
          }

          message.typeStr = reader.string();
          continue;
        case 4:
          if (tag !== 34) {
            break;
          }

          message.data = reader.string();
          continue;
        case 6:
          if (tag !== 48) {
            break;
          }

          message.transactionVersion = longToBigint(reader.uint64() as Long);
          continue;
        case 7:
          if (tag !== 58) {
            break;
          }

          message.transactionTimestamp = Timestamp.decode(reader, reader.uint32());
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
  // Transform<Event, Uint8Array>
  async *encodeTransform(
    source: AsyncIterable<Event | Event[]> | Iterable<Event | Event[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [Event.encode(p).finish()];
        }
      } else {
        yield* [Event.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, Event>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<Event> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [Event.decode(p)];
        }
      } else {
        yield* [Event.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): Event {
    return {
      key: isSet(object.key) ? EventKey.fromJSON(object.key) : undefined,
      sequenceNumber: isSet(object.sequenceNumber) ? BigInt(object.sequenceNumber) : BigInt("0"),
      type: isSet(object.type) ? MoveType.fromJSON(object.type) : undefined,
      typeStr: isSet(object.typeStr) ? globalThis.String(object.typeStr) : "",
      data: isSet(object.data) ? globalThis.String(object.data) : "",
      transactionVersion: isSet(object.transactionVersion) ? BigInt(object.transactionVersion) : BigInt("0"),
      transactionTimestamp: isSet(object.transactionTimestamp)
        ? Timestamp.fromJSON(object.transactionTimestamp)
        : undefined,
    };
  },

  toJSON(message: Event): unknown {
    const obj: any = {};
    if (message.key !== undefined) {
      obj.key = EventKey.toJSON(message.key);
    }
    if (message.sequenceNumber !== undefined && message.sequenceNumber !== BigInt("0")) {
      obj.sequenceNumber = message.sequenceNumber.toString();
    }
    if (message.type !== undefined) {
      obj.type = MoveType.toJSON(message.type);
    }
    if (message.typeStr !== undefined && message.typeStr !== "") {
      obj.typeStr = message.typeStr;
    }
    if (message.data !== undefined && message.data !== "") {
      obj.data = message.data;
    }
    if (message.transactionVersion !== undefined && message.transactionVersion !== BigInt("0")) {
      obj.transactionVersion = message.transactionVersion.toString();
    }
    if (message.transactionTimestamp !== undefined) {
      obj.transactionTimestamp = Timestamp.toJSON(message.transactionTimestamp);
    }
    return obj;
  },

  create(base?: DeepPartial<Event>): Event {
    return Event.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<Event>): Event {
    const message = createBaseEvent();
    message.key = (object.key !== undefined && object.key !== null) ? EventKey.fromPartial(object.key) : undefined;
    message.sequenceNumber = object.sequenceNumber ?? BigInt("0");
    message.type = (object.type !== undefined && object.type !== null) ? MoveType.fromPartial(object.type) : undefined;
    message.typeStr = object.typeStr ?? "";
    message.data = object.data ?? "";
    message.transactionVersion = object.transactionVersion ?? BigInt("0");
    message.transactionTimestamp = (object.transactionTimestamp !== undefined && object.transactionTimestamp !== null)
      ? Timestamp.fromPartial(object.transactionTimestamp)
      : undefined;
    return message;
  },
};

function createBaseEventsInStorage(): EventsInStorage {
  return { transactions: [], startingVersion: undefined };
}

export const EventsInStorage = {
  encode(message: EventsInStorage, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.transactions !== undefined && message.transactions.length !== 0) {
      for (const v of message.transactions) {
        Event.encode(v!, writer.uint32(10).fork()).ldelim();
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

  decode(input: _m0.Reader | Uint8Array, length?: number): EventsInStorage {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseEventsInStorage();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.transactions!.push(Event.decode(reader, reader.uint32()));
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
  // Transform<EventsInStorage, Uint8Array>
  async *encodeTransform(
    source: AsyncIterable<EventsInStorage | EventsInStorage[]> | Iterable<EventsInStorage | EventsInStorage[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [EventsInStorage.encode(p).finish()];
        }
      } else {
        yield* [EventsInStorage.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, EventsInStorage>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<EventsInStorage> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [EventsInStorage.decode(p)];
        }
      } else {
        yield* [EventsInStorage.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): EventsInStorage {
    return {
      transactions: globalThis.Array.isArray(object?.transactions)
        ? object.transactions.map((e: any) => Event.fromJSON(e))
        : [],
      startingVersion: isSet(object.startingVersion) ? BigInt(object.startingVersion) : undefined,
    };
  },

  toJSON(message: EventsInStorage): unknown {
    const obj: any = {};
    if (message.transactions?.length) {
      obj.transactions = message.transactions.map((e) => Event.toJSON(e));
    }
    if (message.startingVersion !== undefined) {
      obj.startingVersion = message.startingVersion.toString();
    }
    return obj;
  },

  create(base?: DeepPartial<EventsInStorage>): EventsInStorage {
    return EventsInStorage.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<EventsInStorage>): EventsInStorage {
    const message = createBaseEventsInStorage();
    message.transactions = object.transactions?.map((e) => Event.fromPartial(e)) || [];
    message.startingVersion = object.startingVersion ?? undefined;
    return message;
  },
};

function createBaseGetEventsRequest(): GetEventsRequest {
  return { startingVersion: undefined, eventsCount: undefined, batchSize: undefined };
}

export const GetEventsRequest = {
  encode(message: GetEventsRequest, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.startingVersion !== undefined) {
      if (BigInt.asUintN(64, message.startingVersion) !== message.startingVersion) {
        throw new globalThis.Error("value provided for field message.startingVersion of type uint64 too large");
      }
      writer.uint32(8).uint64(message.startingVersion.toString());
    }
    if (message.eventsCount !== undefined) {
      if (BigInt.asUintN(64, message.eventsCount) !== message.eventsCount) {
        throw new globalThis.Error("value provided for field message.eventsCount of type uint64 too large");
      }
      writer.uint32(16).uint64(message.eventsCount.toString());
    }
    if (message.batchSize !== undefined) {
      if (BigInt.asUintN(64, message.batchSize) !== message.batchSize) {
        throw new globalThis.Error("value provided for field message.batchSize of type uint64 too large");
      }
      writer.uint32(24).uint64(message.batchSize.toString());
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

          message.eventsCount = longToBigint(reader.uint64() as Long);
          continue;
        case 3:
          if (tag !== 24) {
            break;
          }

          message.batchSize = longToBigint(reader.uint64() as Long);
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
      eventsCount: isSet(object.eventsCount) ? BigInt(object.eventsCount) : undefined,
      batchSize: isSet(object.batchSize) ? BigInt(object.batchSize) : undefined,
    };
  },

  toJSON(message: GetEventsRequest): unknown {
    const obj: any = {};
    if (message.startingVersion !== undefined) {
      obj.startingVersion = message.startingVersion.toString();
    }
    if (message.eventsCount !== undefined) {
      obj.eventsCount = message.eventsCount.toString();
    }
    if (message.batchSize !== undefined) {
      obj.batchSize = message.batchSize.toString();
    }
    return obj;
  },

  create(base?: DeepPartial<GetEventsRequest>): GetEventsRequest {
    return GetEventsRequest.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<GetEventsRequest>): GetEventsRequest {
    const message = createBaseGetEventsRequest();
    message.startingVersion = object.startingVersion ?? undefined;
    message.eventsCount = object.eventsCount ?? undefined;
    message.batchSize = object.batchSize ?? undefined;
    return message;
  },
};

function createBaseEventsResponse(): EventsResponse {
  return { events: [], chainId: undefined };
}

export const EventsResponse = {
  encode(message: EventsResponse, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.events !== undefined && message.events.length !== 0) {
      for (const v of message.events) {
        Event.encode(v!, writer.uint32(10).fork()).ldelim();
      }
    }
    if (message.chainId !== undefined) {
      if (BigInt.asUintN(64, message.chainId) !== message.chainId) {
        throw new globalThis.Error("value provided for field message.chainId of type uint64 too large");
      }
      writer.uint32(16).uint64(message.chainId.toString());
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

          message.events!.push(Event.decode(reader, reader.uint32()));
          continue;
        case 2:
          if (tag !== 16) {
            break;
          }

          message.chainId = longToBigint(reader.uint64() as Long);
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
      events: globalThis.Array.isArray(object?.events) ? object.events.map((e: any) => Event.fromJSON(e)) : [],
      chainId: isSet(object.chainId) ? BigInt(object.chainId) : undefined,
    };
  },

  toJSON(message: EventsResponse): unknown {
    const obj: any = {};
    if (message.events?.length) {
      obj.events = message.events.map((e) => Event.toJSON(e));
    }
    if (message.chainId !== undefined) {
      obj.chainId = message.chainId.toString();
    }
    return obj;
  },

  create(base?: DeepPartial<EventsResponse>): EventsResponse {
    return EventsResponse.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<EventsResponse>): EventsResponse {
    const message = createBaseEventsResponse();
    message.events = object.events?.map((e) => Event.fromPartial(e)) || [];
    message.chainId = object.chainId ?? undefined;
    return message;
  },
};

export type RawEventsService = typeof RawEventsService;
export const RawEventsService = {
  /** Get events batch without any filtering from starting version and end if transaction count is present. */
  getEvents: {
    path: "/aptos.event_stream.v1.RawEvents/GetEvents",
    requestStream: false,
    responseStream: true,
    requestSerialize: (value: GetEventsRequest) => Buffer.from(GetEventsRequest.encode(value).finish()),
    requestDeserialize: (value: Buffer) => GetEventsRequest.decode(value),
    responseSerialize: (value: EventsResponse) => Buffer.from(EventsResponse.encode(value).finish()),
    responseDeserialize: (value: Buffer) => EventsResponse.decode(value),
  },
} as const;

export interface RawEventsServer extends UntypedServiceImplementation {
  /** Get events batch without any filtering from starting version and end if transaction count is present. */
  getEvents: handleServerStreamingCall<GetEventsRequest, EventsResponse>;
}

export interface RawEventsClient extends Client {
  /** Get events batch without any filtering from starting version and end if transaction count is present. */
  getEvents(request: GetEventsRequest, options?: Partial<CallOptions>): ClientReadableStream<EventsResponse>;
  getEvents(
    request: GetEventsRequest,
    metadata?: Metadata,
    options?: Partial<CallOptions>,
  ): ClientReadableStream<EventsResponse>;
}

export const RawEventsClient = makeGenericClientConstructor(
  RawEventsService,
  "aptos.event_stream.v1.RawEvents",
) as unknown as {
  new (address: string, credentials: ChannelCredentials, options?: Partial<ClientOptions>): RawEventsClient;
  service: typeof RawEventsService;
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
