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
import { Timestamp } from "../../util/timestamp/timestamp";
import { GetTransactionsRequest, TransactionsResponse } from "./raw_data";

export interface StreamProgressSampleProto {
  timestamp?: Timestamp | undefined;
  version?: bigint | undefined;
  sizeBytes?: bigint | undefined;
}

export interface StreamProgress {
  samples?: StreamProgressSampleProto[] | undefined;
}

export interface ActiveStream {
  id?: string | undefined;
  startTime?: Timestamp | undefined;
  startVersion?: bigint | undefined;
  endVersion?: bigint | undefined;
  progress?: StreamProgress | undefined;
}

export interface StreamInfo {
  activeStreams?: ActiveStream[] | undefined;
}

export interface LiveDataServiceInfo {
  chainId?: bigint | undefined;
  timestamp?: Timestamp | undefined;
  knownLatestVersion?: bigint | undefined;
  streamInfo?:
    | StreamInfo
    | undefined;
  /** If not present, it means the data service is not available to serve anything yet. */
  minServableVersion?: bigint | undefined;
}

export interface HistoricalDataServiceInfo {
  chainId?: bigint | undefined;
  timestamp?: Timestamp | undefined;
  knownLatestVersion?: bigint | undefined;
  streamInfo?: StreamInfo | undefined;
}

export interface FullnodeInfo {
  chainId?: bigint | undefined;
  timestamp?: Timestamp | undefined;
  knownLatestVersion?: bigint | undefined;
}

export interface GrpcManagerInfo {
  chainId?: bigint | undefined;
  timestamp?: Timestamp | undefined;
  knownLatestVersion?: bigint | undefined;
  masterAddress?: string | undefined;
}

export interface ServiceInfo {
  address?: string | undefined;
  liveDataServiceInfo?: LiveDataServiceInfo | undefined;
  historicalDataServiceInfo?: HistoricalDataServiceInfo | undefined;
  fullnodeInfo?: FullnodeInfo | undefined;
  grpcManagerInfo?: GrpcManagerInfo | undefined;
}

export interface HeartbeatRequest {
  serviceInfo?: ServiceInfo | undefined;
}

export interface HeartbeatResponse {
  knownLatestVersion?: bigint | undefined;
}

export interface PingDataServiceRequest {
  knownLatestVersion?:
    | bigint
    | undefined;
  /** `true` for live data service, `false` for historical data service. */
  pingLiveDataService?: boolean | undefined;
}

export interface PingDataServiceResponse {
  liveDataServiceInfo?: LiveDataServiceInfo | undefined;
  historicalDataServiceInfo?: HistoricalDataServiceInfo | undefined;
}

export interface GetDataServiceForRequestRequest {
  userRequest?: GetTransactionsRequest | undefined;
}

export interface GetDataServiceForRequestResponse {
  dataServiceAddress?: string | undefined;
}

function createBaseStreamProgressSampleProto(): StreamProgressSampleProto {
  return { timestamp: undefined, version: BigInt("0"), sizeBytes: BigInt("0") };
}

export const StreamProgressSampleProto = {
  encode(message: StreamProgressSampleProto, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.timestamp !== undefined) {
      Timestamp.encode(message.timestamp, writer.uint32(10).fork()).ldelim();
    }
    if (message.version !== undefined && message.version !== BigInt("0")) {
      if (BigInt.asUintN(64, message.version) !== message.version) {
        throw new globalThis.Error("value provided for field message.version of type uint64 too large");
      }
      writer.uint32(16).uint64(message.version.toString());
    }
    if (message.sizeBytes !== undefined && message.sizeBytes !== BigInt("0")) {
      if (BigInt.asUintN(64, message.sizeBytes) !== message.sizeBytes) {
        throw new globalThis.Error("value provided for field message.sizeBytes of type uint64 too large");
      }
      writer.uint32(24).uint64(message.sizeBytes.toString());
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): StreamProgressSampleProto {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseStreamProgressSampleProto();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.timestamp = Timestamp.decode(reader, reader.uint32());
          continue;
        case 2:
          if (tag !== 16) {
            break;
          }

          message.version = longToBigint(reader.uint64() as Long);
          continue;
        case 3:
          if (tag !== 24) {
            break;
          }

          message.sizeBytes = longToBigint(reader.uint64() as Long);
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
  // Transform<StreamProgressSampleProto, Uint8Array>
  async *encodeTransform(
    source:
      | AsyncIterable<StreamProgressSampleProto | StreamProgressSampleProto[]>
      | Iterable<StreamProgressSampleProto | StreamProgressSampleProto[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [StreamProgressSampleProto.encode(p).finish()];
        }
      } else {
        yield* [StreamProgressSampleProto.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, StreamProgressSampleProto>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<StreamProgressSampleProto> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [StreamProgressSampleProto.decode(p)];
        }
      } else {
        yield* [StreamProgressSampleProto.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): StreamProgressSampleProto {
    return {
      timestamp: isSet(object.timestamp) ? Timestamp.fromJSON(object.timestamp) : undefined,
      version: isSet(object.version) ? BigInt(object.version) : BigInt("0"),
      sizeBytes: isSet(object.sizeBytes) ? BigInt(object.sizeBytes) : BigInt("0"),
    };
  },

  toJSON(message: StreamProgressSampleProto): unknown {
    const obj: any = {};
    if (message.timestamp !== undefined) {
      obj.timestamp = Timestamp.toJSON(message.timestamp);
    }
    if (message.version !== undefined && message.version !== BigInt("0")) {
      obj.version = message.version.toString();
    }
    if (message.sizeBytes !== undefined && message.sizeBytes !== BigInt("0")) {
      obj.sizeBytes = message.sizeBytes.toString();
    }
    return obj;
  },

  create(base?: DeepPartial<StreamProgressSampleProto>): StreamProgressSampleProto {
    return StreamProgressSampleProto.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<StreamProgressSampleProto>): StreamProgressSampleProto {
    const message = createBaseStreamProgressSampleProto();
    message.timestamp = (object.timestamp !== undefined && object.timestamp !== null)
      ? Timestamp.fromPartial(object.timestamp)
      : undefined;
    message.version = object.version ?? BigInt("0");
    message.sizeBytes = object.sizeBytes ?? BigInt("0");
    return message;
  },
};

function createBaseStreamProgress(): StreamProgress {
  return { samples: [] };
}

export const StreamProgress = {
  encode(message: StreamProgress, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.samples !== undefined && message.samples.length !== 0) {
      for (const v of message.samples) {
        StreamProgressSampleProto.encode(v!, writer.uint32(10).fork()).ldelim();
      }
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): StreamProgress {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseStreamProgress();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.samples!.push(StreamProgressSampleProto.decode(reader, reader.uint32()));
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
  // Transform<StreamProgress, Uint8Array>
  async *encodeTransform(
    source: AsyncIterable<StreamProgress | StreamProgress[]> | Iterable<StreamProgress | StreamProgress[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [StreamProgress.encode(p).finish()];
        }
      } else {
        yield* [StreamProgress.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, StreamProgress>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<StreamProgress> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [StreamProgress.decode(p)];
        }
      } else {
        yield* [StreamProgress.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): StreamProgress {
    return {
      samples: globalThis.Array.isArray(object?.samples)
        ? object.samples.map((e: any) => StreamProgressSampleProto.fromJSON(e))
        : [],
    };
  },

  toJSON(message: StreamProgress): unknown {
    const obj: any = {};
    if (message.samples?.length) {
      obj.samples = message.samples.map((e) => StreamProgressSampleProto.toJSON(e));
    }
    return obj;
  },

  create(base?: DeepPartial<StreamProgress>): StreamProgress {
    return StreamProgress.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<StreamProgress>): StreamProgress {
    const message = createBaseStreamProgress();
    message.samples = object.samples?.map((e) => StreamProgressSampleProto.fromPartial(e)) || [];
    return message;
  },
};

function createBaseActiveStream(): ActiveStream {
  return { id: "", startTime: undefined, startVersion: BigInt("0"), endVersion: undefined, progress: undefined };
}

export const ActiveStream = {
  encode(message: ActiveStream, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.id !== undefined && message.id !== "") {
      writer.uint32(10).string(message.id);
    }
    if (message.startTime !== undefined) {
      Timestamp.encode(message.startTime, writer.uint32(18).fork()).ldelim();
    }
    if (message.startVersion !== undefined && message.startVersion !== BigInt("0")) {
      if (BigInt.asUintN(64, message.startVersion) !== message.startVersion) {
        throw new globalThis.Error("value provided for field message.startVersion of type uint64 too large");
      }
      writer.uint32(24).uint64(message.startVersion.toString());
    }
    if (message.endVersion !== undefined) {
      if (BigInt.asUintN(64, message.endVersion) !== message.endVersion) {
        throw new globalThis.Error("value provided for field message.endVersion of type uint64 too large");
      }
      writer.uint32(32).uint64(message.endVersion.toString());
    }
    if (message.progress !== undefined) {
      StreamProgress.encode(message.progress, writer.uint32(42).fork()).ldelim();
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): ActiveStream {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseActiveStream();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.id = reader.string();
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.startTime = Timestamp.decode(reader, reader.uint32());
          continue;
        case 3:
          if (tag !== 24) {
            break;
          }

          message.startVersion = longToBigint(reader.uint64() as Long);
          continue;
        case 4:
          if (tag !== 32) {
            break;
          }

          message.endVersion = longToBigint(reader.uint64() as Long);
          continue;
        case 5:
          if (tag !== 42) {
            break;
          }

          message.progress = StreamProgress.decode(reader, reader.uint32());
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
  // Transform<ActiveStream, Uint8Array>
  async *encodeTransform(
    source: AsyncIterable<ActiveStream | ActiveStream[]> | Iterable<ActiveStream | ActiveStream[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [ActiveStream.encode(p).finish()];
        }
      } else {
        yield* [ActiveStream.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, ActiveStream>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<ActiveStream> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [ActiveStream.decode(p)];
        }
      } else {
        yield* [ActiveStream.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): ActiveStream {
    return {
      id: isSet(object.id) ? globalThis.String(object.id) : "",
      startTime: isSet(object.startTime) ? Timestamp.fromJSON(object.startTime) : undefined,
      startVersion: isSet(object.startVersion) ? BigInt(object.startVersion) : BigInt("0"),
      endVersion: isSet(object.endVersion) ? BigInt(object.endVersion) : undefined,
      progress: isSet(object.progress) ? StreamProgress.fromJSON(object.progress) : undefined,
    };
  },

  toJSON(message: ActiveStream): unknown {
    const obj: any = {};
    if (message.id !== undefined && message.id !== "") {
      obj.id = message.id;
    }
    if (message.startTime !== undefined) {
      obj.startTime = Timestamp.toJSON(message.startTime);
    }
    if (message.startVersion !== undefined && message.startVersion !== BigInt("0")) {
      obj.startVersion = message.startVersion.toString();
    }
    if (message.endVersion !== undefined) {
      obj.endVersion = message.endVersion.toString();
    }
    if (message.progress !== undefined) {
      obj.progress = StreamProgress.toJSON(message.progress);
    }
    return obj;
  },

  create(base?: DeepPartial<ActiveStream>): ActiveStream {
    return ActiveStream.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<ActiveStream>): ActiveStream {
    const message = createBaseActiveStream();
    message.id = object.id ?? "";
    message.startTime = (object.startTime !== undefined && object.startTime !== null)
      ? Timestamp.fromPartial(object.startTime)
      : undefined;
    message.startVersion = object.startVersion ?? BigInt("0");
    message.endVersion = object.endVersion ?? undefined;
    message.progress = (object.progress !== undefined && object.progress !== null)
      ? StreamProgress.fromPartial(object.progress)
      : undefined;
    return message;
  },
};

function createBaseStreamInfo(): StreamInfo {
  return { activeStreams: [] };
}

export const StreamInfo = {
  encode(message: StreamInfo, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.activeStreams !== undefined && message.activeStreams.length !== 0) {
      for (const v of message.activeStreams) {
        ActiveStream.encode(v!, writer.uint32(10).fork()).ldelim();
      }
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): StreamInfo {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseStreamInfo();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.activeStreams!.push(ActiveStream.decode(reader, reader.uint32()));
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
  // Transform<StreamInfo, Uint8Array>
  async *encodeTransform(
    source: AsyncIterable<StreamInfo | StreamInfo[]> | Iterable<StreamInfo | StreamInfo[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [StreamInfo.encode(p).finish()];
        }
      } else {
        yield* [StreamInfo.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, StreamInfo>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<StreamInfo> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [StreamInfo.decode(p)];
        }
      } else {
        yield* [StreamInfo.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): StreamInfo {
    return {
      activeStreams: globalThis.Array.isArray(object?.activeStreams)
        ? object.activeStreams.map((e: any) => ActiveStream.fromJSON(e))
        : [],
    };
  },

  toJSON(message: StreamInfo): unknown {
    const obj: any = {};
    if (message.activeStreams?.length) {
      obj.activeStreams = message.activeStreams.map((e) => ActiveStream.toJSON(e));
    }
    return obj;
  },

  create(base?: DeepPartial<StreamInfo>): StreamInfo {
    return StreamInfo.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<StreamInfo>): StreamInfo {
    const message = createBaseStreamInfo();
    message.activeStreams = object.activeStreams?.map((e) => ActiveStream.fromPartial(e)) || [];
    return message;
  },
};

function createBaseLiveDataServiceInfo(): LiveDataServiceInfo {
  return {
    chainId: BigInt("0"),
    timestamp: undefined,
    knownLatestVersion: undefined,
    streamInfo: undefined,
    minServableVersion: undefined,
  };
}

export const LiveDataServiceInfo = {
  encode(message: LiveDataServiceInfo, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.chainId !== undefined && message.chainId !== BigInt("0")) {
      if (BigInt.asUintN(64, message.chainId) !== message.chainId) {
        throw new globalThis.Error("value provided for field message.chainId of type uint64 too large");
      }
      writer.uint32(8).uint64(message.chainId.toString());
    }
    if (message.timestamp !== undefined) {
      Timestamp.encode(message.timestamp, writer.uint32(18).fork()).ldelim();
    }
    if (message.knownLatestVersion !== undefined) {
      if (BigInt.asUintN(64, message.knownLatestVersion) !== message.knownLatestVersion) {
        throw new globalThis.Error("value provided for field message.knownLatestVersion of type uint64 too large");
      }
      writer.uint32(24).uint64(message.knownLatestVersion.toString());
    }
    if (message.streamInfo !== undefined) {
      StreamInfo.encode(message.streamInfo, writer.uint32(34).fork()).ldelim();
    }
    if (message.minServableVersion !== undefined) {
      if (BigInt.asUintN(64, message.minServableVersion) !== message.minServableVersion) {
        throw new globalThis.Error("value provided for field message.minServableVersion of type uint64 too large");
      }
      writer.uint32(40).uint64(message.minServableVersion.toString());
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): LiveDataServiceInfo {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseLiveDataServiceInfo();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 8) {
            break;
          }

          message.chainId = longToBigint(reader.uint64() as Long);
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

          message.knownLatestVersion = longToBigint(reader.uint64() as Long);
          continue;
        case 4:
          if (tag !== 34) {
            break;
          }

          message.streamInfo = StreamInfo.decode(reader, reader.uint32());
          continue;
        case 5:
          if (tag !== 40) {
            break;
          }

          message.minServableVersion = longToBigint(reader.uint64() as Long);
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
  // Transform<LiveDataServiceInfo, Uint8Array>
  async *encodeTransform(
    source:
      | AsyncIterable<LiveDataServiceInfo | LiveDataServiceInfo[]>
      | Iterable<LiveDataServiceInfo | LiveDataServiceInfo[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [LiveDataServiceInfo.encode(p).finish()];
        }
      } else {
        yield* [LiveDataServiceInfo.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, LiveDataServiceInfo>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<LiveDataServiceInfo> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [LiveDataServiceInfo.decode(p)];
        }
      } else {
        yield* [LiveDataServiceInfo.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): LiveDataServiceInfo {
    return {
      chainId: isSet(object.chainId) ? BigInt(object.chainId) : BigInt("0"),
      timestamp: isSet(object.timestamp) ? Timestamp.fromJSON(object.timestamp) : undefined,
      knownLatestVersion: isSet(object.knownLatestVersion) ? BigInt(object.knownLatestVersion) : undefined,
      streamInfo: isSet(object.streamInfo) ? StreamInfo.fromJSON(object.streamInfo) : undefined,
      minServableVersion: isSet(object.minServableVersion) ? BigInt(object.minServableVersion) : undefined,
    };
  },

  toJSON(message: LiveDataServiceInfo): unknown {
    const obj: any = {};
    if (message.chainId !== undefined && message.chainId !== BigInt("0")) {
      obj.chainId = message.chainId.toString();
    }
    if (message.timestamp !== undefined) {
      obj.timestamp = Timestamp.toJSON(message.timestamp);
    }
    if (message.knownLatestVersion !== undefined) {
      obj.knownLatestVersion = message.knownLatestVersion.toString();
    }
    if (message.streamInfo !== undefined) {
      obj.streamInfo = StreamInfo.toJSON(message.streamInfo);
    }
    if (message.minServableVersion !== undefined) {
      obj.minServableVersion = message.minServableVersion.toString();
    }
    return obj;
  },

  create(base?: DeepPartial<LiveDataServiceInfo>): LiveDataServiceInfo {
    return LiveDataServiceInfo.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<LiveDataServiceInfo>): LiveDataServiceInfo {
    const message = createBaseLiveDataServiceInfo();
    message.chainId = object.chainId ?? BigInt("0");
    message.timestamp = (object.timestamp !== undefined && object.timestamp !== null)
      ? Timestamp.fromPartial(object.timestamp)
      : undefined;
    message.knownLatestVersion = object.knownLatestVersion ?? undefined;
    message.streamInfo = (object.streamInfo !== undefined && object.streamInfo !== null)
      ? StreamInfo.fromPartial(object.streamInfo)
      : undefined;
    message.minServableVersion = object.minServableVersion ?? undefined;
    return message;
  },
};

function createBaseHistoricalDataServiceInfo(): HistoricalDataServiceInfo {
  return { chainId: BigInt("0"), timestamp: undefined, knownLatestVersion: undefined, streamInfo: undefined };
}

export const HistoricalDataServiceInfo = {
  encode(message: HistoricalDataServiceInfo, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.chainId !== undefined && message.chainId !== BigInt("0")) {
      if (BigInt.asUintN(64, message.chainId) !== message.chainId) {
        throw new globalThis.Error("value provided for field message.chainId of type uint64 too large");
      }
      writer.uint32(8).uint64(message.chainId.toString());
    }
    if (message.timestamp !== undefined) {
      Timestamp.encode(message.timestamp, writer.uint32(18).fork()).ldelim();
    }
    if (message.knownLatestVersion !== undefined) {
      if (BigInt.asUintN(64, message.knownLatestVersion) !== message.knownLatestVersion) {
        throw new globalThis.Error("value provided for field message.knownLatestVersion of type uint64 too large");
      }
      writer.uint32(24).uint64(message.knownLatestVersion.toString());
    }
    if (message.streamInfo !== undefined) {
      StreamInfo.encode(message.streamInfo, writer.uint32(34).fork()).ldelim();
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): HistoricalDataServiceInfo {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseHistoricalDataServiceInfo();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 8) {
            break;
          }

          message.chainId = longToBigint(reader.uint64() as Long);
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

          message.knownLatestVersion = longToBigint(reader.uint64() as Long);
          continue;
        case 4:
          if (tag !== 34) {
            break;
          }

          message.streamInfo = StreamInfo.decode(reader, reader.uint32());
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
  // Transform<HistoricalDataServiceInfo, Uint8Array>
  async *encodeTransform(
    source:
      | AsyncIterable<HistoricalDataServiceInfo | HistoricalDataServiceInfo[]>
      | Iterable<HistoricalDataServiceInfo | HistoricalDataServiceInfo[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [HistoricalDataServiceInfo.encode(p).finish()];
        }
      } else {
        yield* [HistoricalDataServiceInfo.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, HistoricalDataServiceInfo>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<HistoricalDataServiceInfo> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [HistoricalDataServiceInfo.decode(p)];
        }
      } else {
        yield* [HistoricalDataServiceInfo.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): HistoricalDataServiceInfo {
    return {
      chainId: isSet(object.chainId) ? BigInt(object.chainId) : BigInt("0"),
      timestamp: isSet(object.timestamp) ? Timestamp.fromJSON(object.timestamp) : undefined,
      knownLatestVersion: isSet(object.knownLatestVersion) ? BigInt(object.knownLatestVersion) : undefined,
      streamInfo: isSet(object.streamInfo) ? StreamInfo.fromJSON(object.streamInfo) : undefined,
    };
  },

  toJSON(message: HistoricalDataServiceInfo): unknown {
    const obj: any = {};
    if (message.chainId !== undefined && message.chainId !== BigInt("0")) {
      obj.chainId = message.chainId.toString();
    }
    if (message.timestamp !== undefined) {
      obj.timestamp = Timestamp.toJSON(message.timestamp);
    }
    if (message.knownLatestVersion !== undefined) {
      obj.knownLatestVersion = message.knownLatestVersion.toString();
    }
    if (message.streamInfo !== undefined) {
      obj.streamInfo = StreamInfo.toJSON(message.streamInfo);
    }
    return obj;
  },

  create(base?: DeepPartial<HistoricalDataServiceInfo>): HistoricalDataServiceInfo {
    return HistoricalDataServiceInfo.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<HistoricalDataServiceInfo>): HistoricalDataServiceInfo {
    const message = createBaseHistoricalDataServiceInfo();
    message.chainId = object.chainId ?? BigInt("0");
    message.timestamp = (object.timestamp !== undefined && object.timestamp !== null)
      ? Timestamp.fromPartial(object.timestamp)
      : undefined;
    message.knownLatestVersion = object.knownLatestVersion ?? undefined;
    message.streamInfo = (object.streamInfo !== undefined && object.streamInfo !== null)
      ? StreamInfo.fromPartial(object.streamInfo)
      : undefined;
    return message;
  },
};

function createBaseFullnodeInfo(): FullnodeInfo {
  return { chainId: BigInt("0"), timestamp: undefined, knownLatestVersion: undefined };
}

export const FullnodeInfo = {
  encode(message: FullnodeInfo, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.chainId !== undefined && message.chainId !== BigInt("0")) {
      if (BigInt.asUintN(64, message.chainId) !== message.chainId) {
        throw new globalThis.Error("value provided for field message.chainId of type uint64 too large");
      }
      writer.uint32(8).uint64(message.chainId.toString());
    }
    if (message.timestamp !== undefined) {
      Timestamp.encode(message.timestamp, writer.uint32(18).fork()).ldelim();
    }
    if (message.knownLatestVersion !== undefined) {
      if (BigInt.asUintN(64, message.knownLatestVersion) !== message.knownLatestVersion) {
        throw new globalThis.Error("value provided for field message.knownLatestVersion of type uint64 too large");
      }
      writer.uint32(24).uint64(message.knownLatestVersion.toString());
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): FullnodeInfo {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseFullnodeInfo();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 8) {
            break;
          }

          message.chainId = longToBigint(reader.uint64() as Long);
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

          message.knownLatestVersion = longToBigint(reader.uint64() as Long);
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
  // Transform<FullnodeInfo, Uint8Array>
  async *encodeTransform(
    source: AsyncIterable<FullnodeInfo | FullnodeInfo[]> | Iterable<FullnodeInfo | FullnodeInfo[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [FullnodeInfo.encode(p).finish()];
        }
      } else {
        yield* [FullnodeInfo.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, FullnodeInfo>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<FullnodeInfo> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [FullnodeInfo.decode(p)];
        }
      } else {
        yield* [FullnodeInfo.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): FullnodeInfo {
    return {
      chainId: isSet(object.chainId) ? BigInt(object.chainId) : BigInt("0"),
      timestamp: isSet(object.timestamp) ? Timestamp.fromJSON(object.timestamp) : undefined,
      knownLatestVersion: isSet(object.knownLatestVersion) ? BigInt(object.knownLatestVersion) : undefined,
    };
  },

  toJSON(message: FullnodeInfo): unknown {
    const obj: any = {};
    if (message.chainId !== undefined && message.chainId !== BigInt("0")) {
      obj.chainId = message.chainId.toString();
    }
    if (message.timestamp !== undefined) {
      obj.timestamp = Timestamp.toJSON(message.timestamp);
    }
    if (message.knownLatestVersion !== undefined) {
      obj.knownLatestVersion = message.knownLatestVersion.toString();
    }
    return obj;
  },

  create(base?: DeepPartial<FullnodeInfo>): FullnodeInfo {
    return FullnodeInfo.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<FullnodeInfo>): FullnodeInfo {
    const message = createBaseFullnodeInfo();
    message.chainId = object.chainId ?? BigInt("0");
    message.timestamp = (object.timestamp !== undefined && object.timestamp !== null)
      ? Timestamp.fromPartial(object.timestamp)
      : undefined;
    message.knownLatestVersion = object.knownLatestVersion ?? undefined;
    return message;
  },
};

function createBaseGrpcManagerInfo(): GrpcManagerInfo {
  return { chainId: BigInt("0"), timestamp: undefined, knownLatestVersion: undefined, masterAddress: undefined };
}

export const GrpcManagerInfo = {
  encode(message: GrpcManagerInfo, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.chainId !== undefined && message.chainId !== BigInt("0")) {
      if (BigInt.asUintN(64, message.chainId) !== message.chainId) {
        throw new globalThis.Error("value provided for field message.chainId of type uint64 too large");
      }
      writer.uint32(8).uint64(message.chainId.toString());
    }
    if (message.timestamp !== undefined) {
      Timestamp.encode(message.timestamp, writer.uint32(18).fork()).ldelim();
    }
    if (message.knownLatestVersion !== undefined) {
      if (BigInt.asUintN(64, message.knownLatestVersion) !== message.knownLatestVersion) {
        throw new globalThis.Error("value provided for field message.knownLatestVersion of type uint64 too large");
      }
      writer.uint32(24).uint64(message.knownLatestVersion.toString());
    }
    if (message.masterAddress !== undefined) {
      writer.uint32(34).string(message.masterAddress);
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): GrpcManagerInfo {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseGrpcManagerInfo();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 8) {
            break;
          }

          message.chainId = longToBigint(reader.uint64() as Long);
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

          message.knownLatestVersion = longToBigint(reader.uint64() as Long);
          continue;
        case 4:
          if (tag !== 34) {
            break;
          }

          message.masterAddress = reader.string();
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
  // Transform<GrpcManagerInfo, Uint8Array>
  async *encodeTransform(
    source: AsyncIterable<GrpcManagerInfo | GrpcManagerInfo[]> | Iterable<GrpcManagerInfo | GrpcManagerInfo[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [GrpcManagerInfo.encode(p).finish()];
        }
      } else {
        yield* [GrpcManagerInfo.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, GrpcManagerInfo>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<GrpcManagerInfo> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [GrpcManagerInfo.decode(p)];
        }
      } else {
        yield* [GrpcManagerInfo.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): GrpcManagerInfo {
    return {
      chainId: isSet(object.chainId) ? BigInt(object.chainId) : BigInt("0"),
      timestamp: isSet(object.timestamp) ? Timestamp.fromJSON(object.timestamp) : undefined,
      knownLatestVersion: isSet(object.knownLatestVersion) ? BigInt(object.knownLatestVersion) : undefined,
      masterAddress: isSet(object.masterAddress) ? globalThis.String(object.masterAddress) : undefined,
    };
  },

  toJSON(message: GrpcManagerInfo): unknown {
    const obj: any = {};
    if (message.chainId !== undefined && message.chainId !== BigInt("0")) {
      obj.chainId = message.chainId.toString();
    }
    if (message.timestamp !== undefined) {
      obj.timestamp = Timestamp.toJSON(message.timestamp);
    }
    if (message.knownLatestVersion !== undefined) {
      obj.knownLatestVersion = message.knownLatestVersion.toString();
    }
    if (message.masterAddress !== undefined) {
      obj.masterAddress = message.masterAddress;
    }
    return obj;
  },

  create(base?: DeepPartial<GrpcManagerInfo>): GrpcManagerInfo {
    return GrpcManagerInfo.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<GrpcManagerInfo>): GrpcManagerInfo {
    const message = createBaseGrpcManagerInfo();
    message.chainId = object.chainId ?? BigInt("0");
    message.timestamp = (object.timestamp !== undefined && object.timestamp !== null)
      ? Timestamp.fromPartial(object.timestamp)
      : undefined;
    message.knownLatestVersion = object.knownLatestVersion ?? undefined;
    message.masterAddress = object.masterAddress ?? undefined;
    return message;
  },
};

function createBaseServiceInfo(): ServiceInfo {
  return {
    address: undefined,
    liveDataServiceInfo: undefined,
    historicalDataServiceInfo: undefined,
    fullnodeInfo: undefined,
    grpcManagerInfo: undefined,
  };
}

export const ServiceInfo = {
  encode(message: ServiceInfo, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.address !== undefined) {
      writer.uint32(10).string(message.address);
    }
    if (message.liveDataServiceInfo !== undefined) {
      LiveDataServiceInfo.encode(message.liveDataServiceInfo, writer.uint32(18).fork()).ldelim();
    }
    if (message.historicalDataServiceInfo !== undefined) {
      HistoricalDataServiceInfo.encode(message.historicalDataServiceInfo, writer.uint32(26).fork()).ldelim();
    }
    if (message.fullnodeInfo !== undefined) {
      FullnodeInfo.encode(message.fullnodeInfo, writer.uint32(34).fork()).ldelim();
    }
    if (message.grpcManagerInfo !== undefined) {
      GrpcManagerInfo.encode(message.grpcManagerInfo, writer.uint32(42).fork()).ldelim();
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): ServiceInfo {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseServiceInfo();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.address = reader.string();
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.liveDataServiceInfo = LiveDataServiceInfo.decode(reader, reader.uint32());
          continue;
        case 3:
          if (tag !== 26) {
            break;
          }

          message.historicalDataServiceInfo = HistoricalDataServiceInfo.decode(reader, reader.uint32());
          continue;
        case 4:
          if (tag !== 34) {
            break;
          }

          message.fullnodeInfo = FullnodeInfo.decode(reader, reader.uint32());
          continue;
        case 5:
          if (tag !== 42) {
            break;
          }

          message.grpcManagerInfo = GrpcManagerInfo.decode(reader, reader.uint32());
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
  // Transform<ServiceInfo, Uint8Array>
  async *encodeTransform(
    source: AsyncIterable<ServiceInfo | ServiceInfo[]> | Iterable<ServiceInfo | ServiceInfo[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [ServiceInfo.encode(p).finish()];
        }
      } else {
        yield* [ServiceInfo.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, ServiceInfo>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<ServiceInfo> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [ServiceInfo.decode(p)];
        }
      } else {
        yield* [ServiceInfo.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): ServiceInfo {
    return {
      address: isSet(object.address) ? globalThis.String(object.address) : undefined,
      liveDataServiceInfo: isSet(object.liveDataServiceInfo)
        ? LiveDataServiceInfo.fromJSON(object.liveDataServiceInfo)
        : undefined,
      historicalDataServiceInfo: isSet(object.historicalDataServiceInfo)
        ? HistoricalDataServiceInfo.fromJSON(object.historicalDataServiceInfo)
        : undefined,
      fullnodeInfo: isSet(object.fullnodeInfo) ? FullnodeInfo.fromJSON(object.fullnodeInfo) : undefined,
      grpcManagerInfo: isSet(object.grpcManagerInfo) ? GrpcManagerInfo.fromJSON(object.grpcManagerInfo) : undefined,
    };
  },

  toJSON(message: ServiceInfo): unknown {
    const obj: any = {};
    if (message.address !== undefined) {
      obj.address = message.address;
    }
    if (message.liveDataServiceInfo !== undefined) {
      obj.liveDataServiceInfo = LiveDataServiceInfo.toJSON(message.liveDataServiceInfo);
    }
    if (message.historicalDataServiceInfo !== undefined) {
      obj.historicalDataServiceInfo = HistoricalDataServiceInfo.toJSON(message.historicalDataServiceInfo);
    }
    if (message.fullnodeInfo !== undefined) {
      obj.fullnodeInfo = FullnodeInfo.toJSON(message.fullnodeInfo);
    }
    if (message.grpcManagerInfo !== undefined) {
      obj.grpcManagerInfo = GrpcManagerInfo.toJSON(message.grpcManagerInfo);
    }
    return obj;
  },

  create(base?: DeepPartial<ServiceInfo>): ServiceInfo {
    return ServiceInfo.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<ServiceInfo>): ServiceInfo {
    const message = createBaseServiceInfo();
    message.address = object.address ?? undefined;
    message.liveDataServiceInfo = (object.liveDataServiceInfo !== undefined && object.liveDataServiceInfo !== null)
      ? LiveDataServiceInfo.fromPartial(object.liveDataServiceInfo)
      : undefined;
    message.historicalDataServiceInfo =
      (object.historicalDataServiceInfo !== undefined && object.historicalDataServiceInfo !== null)
        ? HistoricalDataServiceInfo.fromPartial(object.historicalDataServiceInfo)
        : undefined;
    message.fullnodeInfo = (object.fullnodeInfo !== undefined && object.fullnodeInfo !== null)
      ? FullnodeInfo.fromPartial(object.fullnodeInfo)
      : undefined;
    message.grpcManagerInfo = (object.grpcManagerInfo !== undefined && object.grpcManagerInfo !== null)
      ? GrpcManagerInfo.fromPartial(object.grpcManagerInfo)
      : undefined;
    return message;
  },
};

function createBaseHeartbeatRequest(): HeartbeatRequest {
  return { serviceInfo: undefined };
}

export const HeartbeatRequest = {
  encode(message: HeartbeatRequest, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.serviceInfo !== undefined) {
      ServiceInfo.encode(message.serviceInfo, writer.uint32(10).fork()).ldelim();
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): HeartbeatRequest {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseHeartbeatRequest();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.serviceInfo = ServiceInfo.decode(reader, reader.uint32());
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
  // Transform<HeartbeatRequest, Uint8Array>
  async *encodeTransform(
    source: AsyncIterable<HeartbeatRequest | HeartbeatRequest[]> | Iterable<HeartbeatRequest | HeartbeatRequest[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [HeartbeatRequest.encode(p).finish()];
        }
      } else {
        yield* [HeartbeatRequest.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, HeartbeatRequest>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<HeartbeatRequest> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [HeartbeatRequest.decode(p)];
        }
      } else {
        yield* [HeartbeatRequest.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): HeartbeatRequest {
    return { serviceInfo: isSet(object.serviceInfo) ? ServiceInfo.fromJSON(object.serviceInfo) : undefined };
  },

  toJSON(message: HeartbeatRequest): unknown {
    const obj: any = {};
    if (message.serviceInfo !== undefined) {
      obj.serviceInfo = ServiceInfo.toJSON(message.serviceInfo);
    }
    return obj;
  },

  create(base?: DeepPartial<HeartbeatRequest>): HeartbeatRequest {
    return HeartbeatRequest.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<HeartbeatRequest>): HeartbeatRequest {
    const message = createBaseHeartbeatRequest();
    message.serviceInfo = (object.serviceInfo !== undefined && object.serviceInfo !== null)
      ? ServiceInfo.fromPartial(object.serviceInfo)
      : undefined;
    return message;
  },
};

function createBaseHeartbeatResponse(): HeartbeatResponse {
  return { knownLatestVersion: undefined };
}

export const HeartbeatResponse = {
  encode(message: HeartbeatResponse, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.knownLatestVersion !== undefined) {
      if (BigInt.asUintN(64, message.knownLatestVersion) !== message.knownLatestVersion) {
        throw new globalThis.Error("value provided for field message.knownLatestVersion of type uint64 too large");
      }
      writer.uint32(8).uint64(message.knownLatestVersion.toString());
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): HeartbeatResponse {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseHeartbeatResponse();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 8) {
            break;
          }

          message.knownLatestVersion = longToBigint(reader.uint64() as Long);
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
  // Transform<HeartbeatResponse, Uint8Array>
  async *encodeTransform(
    source: AsyncIterable<HeartbeatResponse | HeartbeatResponse[]> | Iterable<HeartbeatResponse | HeartbeatResponse[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [HeartbeatResponse.encode(p).finish()];
        }
      } else {
        yield* [HeartbeatResponse.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, HeartbeatResponse>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<HeartbeatResponse> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [HeartbeatResponse.decode(p)];
        }
      } else {
        yield* [HeartbeatResponse.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): HeartbeatResponse {
    return { knownLatestVersion: isSet(object.knownLatestVersion) ? BigInt(object.knownLatestVersion) : undefined };
  },

  toJSON(message: HeartbeatResponse): unknown {
    const obj: any = {};
    if (message.knownLatestVersion !== undefined) {
      obj.knownLatestVersion = message.knownLatestVersion.toString();
    }
    return obj;
  },

  create(base?: DeepPartial<HeartbeatResponse>): HeartbeatResponse {
    return HeartbeatResponse.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<HeartbeatResponse>): HeartbeatResponse {
    const message = createBaseHeartbeatResponse();
    message.knownLatestVersion = object.knownLatestVersion ?? undefined;
    return message;
  },
};

function createBasePingDataServiceRequest(): PingDataServiceRequest {
  return { knownLatestVersion: undefined, pingLiveDataService: false };
}

export const PingDataServiceRequest = {
  encode(message: PingDataServiceRequest, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.knownLatestVersion !== undefined) {
      if (BigInt.asUintN(64, message.knownLatestVersion) !== message.knownLatestVersion) {
        throw new globalThis.Error("value provided for field message.knownLatestVersion of type uint64 too large");
      }
      writer.uint32(8).uint64(message.knownLatestVersion.toString());
    }
    if (message.pingLiveDataService === true) {
      writer.uint32(16).bool(message.pingLiveDataService);
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): PingDataServiceRequest {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBasePingDataServiceRequest();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 8) {
            break;
          }

          message.knownLatestVersion = longToBigint(reader.uint64() as Long);
          continue;
        case 2:
          if (tag !== 16) {
            break;
          }

          message.pingLiveDataService = reader.bool();
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
  // Transform<PingDataServiceRequest, Uint8Array>
  async *encodeTransform(
    source:
      | AsyncIterable<PingDataServiceRequest | PingDataServiceRequest[]>
      | Iterable<PingDataServiceRequest | PingDataServiceRequest[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [PingDataServiceRequest.encode(p).finish()];
        }
      } else {
        yield* [PingDataServiceRequest.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, PingDataServiceRequest>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<PingDataServiceRequest> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [PingDataServiceRequest.decode(p)];
        }
      } else {
        yield* [PingDataServiceRequest.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): PingDataServiceRequest {
    return {
      knownLatestVersion: isSet(object.knownLatestVersion) ? BigInt(object.knownLatestVersion) : undefined,
      pingLiveDataService: isSet(object.pingLiveDataService) ? globalThis.Boolean(object.pingLiveDataService) : false,
    };
  },

  toJSON(message: PingDataServiceRequest): unknown {
    const obj: any = {};
    if (message.knownLatestVersion !== undefined) {
      obj.knownLatestVersion = message.knownLatestVersion.toString();
    }
    if (message.pingLiveDataService === true) {
      obj.pingLiveDataService = message.pingLiveDataService;
    }
    return obj;
  },

  create(base?: DeepPartial<PingDataServiceRequest>): PingDataServiceRequest {
    return PingDataServiceRequest.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<PingDataServiceRequest>): PingDataServiceRequest {
    const message = createBasePingDataServiceRequest();
    message.knownLatestVersion = object.knownLatestVersion ?? undefined;
    message.pingLiveDataService = object.pingLiveDataService ?? false;
    return message;
  },
};

function createBasePingDataServiceResponse(): PingDataServiceResponse {
  return { liveDataServiceInfo: undefined, historicalDataServiceInfo: undefined };
}

export const PingDataServiceResponse = {
  encode(message: PingDataServiceResponse, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.liveDataServiceInfo !== undefined) {
      LiveDataServiceInfo.encode(message.liveDataServiceInfo, writer.uint32(10).fork()).ldelim();
    }
    if (message.historicalDataServiceInfo !== undefined) {
      HistoricalDataServiceInfo.encode(message.historicalDataServiceInfo, writer.uint32(18).fork()).ldelim();
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): PingDataServiceResponse {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBasePingDataServiceResponse();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.liveDataServiceInfo = LiveDataServiceInfo.decode(reader, reader.uint32());
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.historicalDataServiceInfo = HistoricalDataServiceInfo.decode(reader, reader.uint32());
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
  // Transform<PingDataServiceResponse, Uint8Array>
  async *encodeTransform(
    source:
      | AsyncIterable<PingDataServiceResponse | PingDataServiceResponse[]>
      | Iterable<PingDataServiceResponse | PingDataServiceResponse[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [PingDataServiceResponse.encode(p).finish()];
        }
      } else {
        yield* [PingDataServiceResponse.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, PingDataServiceResponse>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<PingDataServiceResponse> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [PingDataServiceResponse.decode(p)];
        }
      } else {
        yield* [PingDataServiceResponse.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): PingDataServiceResponse {
    return {
      liveDataServiceInfo: isSet(object.liveDataServiceInfo)
        ? LiveDataServiceInfo.fromJSON(object.liveDataServiceInfo)
        : undefined,
      historicalDataServiceInfo: isSet(object.historicalDataServiceInfo)
        ? HistoricalDataServiceInfo.fromJSON(object.historicalDataServiceInfo)
        : undefined,
    };
  },

  toJSON(message: PingDataServiceResponse): unknown {
    const obj: any = {};
    if (message.liveDataServiceInfo !== undefined) {
      obj.liveDataServiceInfo = LiveDataServiceInfo.toJSON(message.liveDataServiceInfo);
    }
    if (message.historicalDataServiceInfo !== undefined) {
      obj.historicalDataServiceInfo = HistoricalDataServiceInfo.toJSON(message.historicalDataServiceInfo);
    }
    return obj;
  },

  create(base?: DeepPartial<PingDataServiceResponse>): PingDataServiceResponse {
    return PingDataServiceResponse.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<PingDataServiceResponse>): PingDataServiceResponse {
    const message = createBasePingDataServiceResponse();
    message.liveDataServiceInfo = (object.liveDataServiceInfo !== undefined && object.liveDataServiceInfo !== null)
      ? LiveDataServiceInfo.fromPartial(object.liveDataServiceInfo)
      : undefined;
    message.historicalDataServiceInfo =
      (object.historicalDataServiceInfo !== undefined && object.historicalDataServiceInfo !== null)
        ? HistoricalDataServiceInfo.fromPartial(object.historicalDataServiceInfo)
        : undefined;
    return message;
  },
};

function createBaseGetDataServiceForRequestRequest(): GetDataServiceForRequestRequest {
  return { userRequest: undefined };
}

export const GetDataServiceForRequestRequest = {
  encode(message: GetDataServiceForRequestRequest, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.userRequest !== undefined) {
      GetTransactionsRequest.encode(message.userRequest, writer.uint32(10).fork()).ldelim();
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): GetDataServiceForRequestRequest {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseGetDataServiceForRequestRequest();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.userRequest = GetTransactionsRequest.decode(reader, reader.uint32());
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
  // Transform<GetDataServiceForRequestRequest, Uint8Array>
  async *encodeTransform(
    source:
      | AsyncIterable<GetDataServiceForRequestRequest | GetDataServiceForRequestRequest[]>
      | Iterable<GetDataServiceForRequestRequest | GetDataServiceForRequestRequest[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [GetDataServiceForRequestRequest.encode(p).finish()];
        }
      } else {
        yield* [GetDataServiceForRequestRequest.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, GetDataServiceForRequestRequest>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<GetDataServiceForRequestRequest> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [GetDataServiceForRequestRequest.decode(p)];
        }
      } else {
        yield* [GetDataServiceForRequestRequest.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): GetDataServiceForRequestRequest {
    return { userRequest: isSet(object.userRequest) ? GetTransactionsRequest.fromJSON(object.userRequest) : undefined };
  },

  toJSON(message: GetDataServiceForRequestRequest): unknown {
    const obj: any = {};
    if (message.userRequest !== undefined) {
      obj.userRequest = GetTransactionsRequest.toJSON(message.userRequest);
    }
    return obj;
  },

  create(base?: DeepPartial<GetDataServiceForRequestRequest>): GetDataServiceForRequestRequest {
    return GetDataServiceForRequestRequest.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<GetDataServiceForRequestRequest>): GetDataServiceForRequestRequest {
    const message = createBaseGetDataServiceForRequestRequest();
    message.userRequest = (object.userRequest !== undefined && object.userRequest !== null)
      ? GetTransactionsRequest.fromPartial(object.userRequest)
      : undefined;
    return message;
  },
};

function createBaseGetDataServiceForRequestResponse(): GetDataServiceForRequestResponse {
  return { dataServiceAddress: "" };
}

export const GetDataServiceForRequestResponse = {
  encode(message: GetDataServiceForRequestResponse, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.dataServiceAddress !== undefined && message.dataServiceAddress !== "") {
      writer.uint32(10).string(message.dataServiceAddress);
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): GetDataServiceForRequestResponse {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseGetDataServiceForRequestResponse();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.dataServiceAddress = reader.string();
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
  // Transform<GetDataServiceForRequestResponse, Uint8Array>
  async *encodeTransform(
    source:
      | AsyncIterable<GetDataServiceForRequestResponse | GetDataServiceForRequestResponse[]>
      | Iterable<GetDataServiceForRequestResponse | GetDataServiceForRequestResponse[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [GetDataServiceForRequestResponse.encode(p).finish()];
        }
      } else {
        yield* [GetDataServiceForRequestResponse.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, GetDataServiceForRequestResponse>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<GetDataServiceForRequestResponse> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [GetDataServiceForRequestResponse.decode(p)];
        }
      } else {
        yield* [GetDataServiceForRequestResponse.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): GetDataServiceForRequestResponse {
    return { dataServiceAddress: isSet(object.dataServiceAddress) ? globalThis.String(object.dataServiceAddress) : "" };
  },

  toJSON(message: GetDataServiceForRequestResponse): unknown {
    const obj: any = {};
    if (message.dataServiceAddress !== undefined && message.dataServiceAddress !== "") {
      obj.dataServiceAddress = message.dataServiceAddress;
    }
    return obj;
  },

  create(base?: DeepPartial<GetDataServiceForRequestResponse>): GetDataServiceForRequestResponse {
    return GetDataServiceForRequestResponse.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<GetDataServiceForRequestResponse>): GetDataServiceForRequestResponse {
    const message = createBaseGetDataServiceForRequestResponse();
    message.dataServiceAddress = object.dataServiceAddress ?? "";
    return message;
  },
};

export type GrpcManagerService = typeof GrpcManagerService;
export const GrpcManagerService = {
  heartbeat: {
    path: "/velor.indexer.v1.GrpcManager/Heartbeat",
    requestStream: false,
    responseStream: false,
    requestSerialize: (value: HeartbeatRequest) => Buffer.from(HeartbeatRequest.encode(value).finish()),
    requestDeserialize: (value: Buffer) => HeartbeatRequest.decode(value),
    responseSerialize: (value: HeartbeatResponse) => Buffer.from(HeartbeatResponse.encode(value).finish()),
    responseDeserialize: (value: Buffer) => HeartbeatResponse.decode(value),
  },
  getTransactions: {
    path: "/velor.indexer.v1.GrpcManager/GetTransactions",
    requestStream: false,
    responseStream: false,
    requestSerialize: (value: GetTransactionsRequest) => Buffer.from(GetTransactionsRequest.encode(value).finish()),
    requestDeserialize: (value: Buffer) => GetTransactionsRequest.decode(value),
    responseSerialize: (value: TransactionsResponse) => Buffer.from(TransactionsResponse.encode(value).finish()),
    responseDeserialize: (value: Buffer) => TransactionsResponse.decode(value),
  },
  getDataServiceForRequest: {
    path: "/velor.indexer.v1.GrpcManager/GetDataServiceForRequest",
    requestStream: false,
    responseStream: false,
    requestSerialize: (value: GetDataServiceForRequestRequest) =>
      Buffer.from(GetDataServiceForRequestRequest.encode(value).finish()),
    requestDeserialize: (value: Buffer) => GetDataServiceForRequestRequest.decode(value),
    responseSerialize: (value: GetDataServiceForRequestResponse) =>
      Buffer.from(GetDataServiceForRequestResponse.encode(value).finish()),
    responseDeserialize: (value: Buffer) => GetDataServiceForRequestResponse.decode(value),
  },
} as const;

export interface GrpcManagerServer extends UntypedServiceImplementation {
  heartbeat: handleUnaryCall<HeartbeatRequest, HeartbeatResponse>;
  getTransactions: handleUnaryCall<GetTransactionsRequest, TransactionsResponse>;
  getDataServiceForRequest: handleUnaryCall<GetDataServiceForRequestRequest, GetDataServiceForRequestResponse>;
}

export interface GrpcManagerClient extends Client {
  heartbeat(
    request: HeartbeatRequest,
    callback: (error: ServiceError | null, response: HeartbeatResponse) => void,
  ): ClientUnaryCall;
  heartbeat(
    request: HeartbeatRequest,
    metadata: Metadata,
    callback: (error: ServiceError | null, response: HeartbeatResponse) => void,
  ): ClientUnaryCall;
  heartbeat(
    request: HeartbeatRequest,
    metadata: Metadata,
    options: Partial<CallOptions>,
    callback: (error: ServiceError | null, response: HeartbeatResponse) => void,
  ): ClientUnaryCall;
  getTransactions(
    request: GetTransactionsRequest,
    callback: (error: ServiceError | null, response: TransactionsResponse) => void,
  ): ClientUnaryCall;
  getTransactions(
    request: GetTransactionsRequest,
    metadata: Metadata,
    callback: (error: ServiceError | null, response: TransactionsResponse) => void,
  ): ClientUnaryCall;
  getTransactions(
    request: GetTransactionsRequest,
    metadata: Metadata,
    options: Partial<CallOptions>,
    callback: (error: ServiceError | null, response: TransactionsResponse) => void,
  ): ClientUnaryCall;
  getDataServiceForRequest(
    request: GetDataServiceForRequestRequest,
    callback: (error: ServiceError | null, response: GetDataServiceForRequestResponse) => void,
  ): ClientUnaryCall;
  getDataServiceForRequest(
    request: GetDataServiceForRequestRequest,
    metadata: Metadata,
    callback: (error: ServiceError | null, response: GetDataServiceForRequestResponse) => void,
  ): ClientUnaryCall;
  getDataServiceForRequest(
    request: GetDataServiceForRequestRequest,
    metadata: Metadata,
    options: Partial<CallOptions>,
    callback: (error: ServiceError | null, response: GetDataServiceForRequestResponse) => void,
  ): ClientUnaryCall;
}

export const GrpcManagerClient = makeGenericClientConstructor(
  GrpcManagerService,
  "velor.indexer.v1.GrpcManager",
) as unknown as {
  new (address: string, credentials: ChannelCredentials, options?: Partial<ClientOptions>): GrpcManagerClient;
  service: typeof GrpcManagerService;
  serviceName: string;
};

export type DataServiceService = typeof DataServiceService;
export const DataServiceService = {
  ping: {
    path: "/velor.indexer.v1.DataService/Ping",
    requestStream: false,
    responseStream: false,
    requestSerialize: (value: PingDataServiceRequest) => Buffer.from(PingDataServiceRequest.encode(value).finish()),
    requestDeserialize: (value: Buffer) => PingDataServiceRequest.decode(value),
    responseSerialize: (value: PingDataServiceResponse) => Buffer.from(PingDataServiceResponse.encode(value).finish()),
    responseDeserialize: (value: Buffer) => PingDataServiceResponse.decode(value),
  },
  getTransactions: {
    path: "/velor.indexer.v1.DataService/GetTransactions",
    requestStream: false,
    responseStream: true,
    requestSerialize: (value: GetTransactionsRequest) => Buffer.from(GetTransactionsRequest.encode(value).finish()),
    requestDeserialize: (value: Buffer) => GetTransactionsRequest.decode(value),
    responseSerialize: (value: TransactionsResponse) => Buffer.from(TransactionsResponse.encode(value).finish()),
    responseDeserialize: (value: Buffer) => TransactionsResponse.decode(value),
  },
} as const;

export interface DataServiceServer extends UntypedServiceImplementation {
  ping: handleUnaryCall<PingDataServiceRequest, PingDataServiceResponse>;
  getTransactions: handleServerStreamingCall<GetTransactionsRequest, TransactionsResponse>;
}

export interface DataServiceClient extends Client {
  ping(
    request: PingDataServiceRequest,
    callback: (error: ServiceError | null, response: PingDataServiceResponse) => void,
  ): ClientUnaryCall;
  ping(
    request: PingDataServiceRequest,
    metadata: Metadata,
    callback: (error: ServiceError | null, response: PingDataServiceResponse) => void,
  ): ClientUnaryCall;
  ping(
    request: PingDataServiceRequest,
    metadata: Metadata,
    options: Partial<CallOptions>,
    callback: (error: ServiceError | null, response: PingDataServiceResponse) => void,
  ): ClientUnaryCall;
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

export const DataServiceClient = makeGenericClientConstructor(
  DataServiceService,
  "velor.indexer.v1.DataService",
) as unknown as {
  new (address: string, credentials: ChannelCredentials, options?: Partial<ClientOptions>): DataServiceClient;
  service: typeof DataServiceService;
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
