/* eslint-disable */
import { ChannelCredentials, Client, makeGenericClientConstructor, Metadata } from "@grpc/grpc-js";
import type {
  CallOptions,
  ClientOptions,
  ClientUnaryCall,
  handleUnaryCall,
  ServiceError,
  UntypedServiceImplementation,
} from "@grpc/grpc-js";
import _m0 from "protobufjs/minimal";

export interface NetworkMessage {
  message?: Uint8Array | undefined;
  messageType?: string | undefined;
}

export interface Empty {
}

function createBaseNetworkMessage(): NetworkMessage {
  return { message: new Uint8Array(0), messageType: "" };
}

export const NetworkMessage = {
  encode(message: NetworkMessage, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.message !== undefined && message.message.length !== 0) {
      writer.uint32(10).bytes(message.message);
    }
    if (message.messageType !== undefined && message.messageType !== "") {
      writer.uint32(18).string(message.messageType);
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): NetworkMessage {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseNetworkMessage();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.message = reader.bytes();
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.messageType = reader.string();
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
  // Transform<NetworkMessage, Uint8Array>
  async *encodeTransform(
    source: AsyncIterable<NetworkMessage | NetworkMessage[]> | Iterable<NetworkMessage | NetworkMessage[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [NetworkMessage.encode(p).finish()];
        }
      } else {
        yield* [NetworkMessage.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, NetworkMessage>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<NetworkMessage> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [NetworkMessage.decode(p)];
        }
      } else {
        yield* [NetworkMessage.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): NetworkMessage {
    return {
      message: isSet(object.message) ? bytesFromBase64(object.message) : new Uint8Array(0),
      messageType: isSet(object.messageType) ? globalThis.String(object.messageType) : "",
    };
  },

  toJSON(message: NetworkMessage): unknown {
    const obj: any = {};
    if (message.message !== undefined && message.message.length !== 0) {
      obj.message = base64FromBytes(message.message);
    }
    if (message.messageType !== undefined && message.messageType !== "") {
      obj.messageType = message.messageType;
    }
    return obj;
  },

  create(base?: DeepPartial<NetworkMessage>): NetworkMessage {
    return NetworkMessage.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<NetworkMessage>): NetworkMessage {
    const message = createBaseNetworkMessage();
    message.message = object.message ?? new Uint8Array(0);
    message.messageType = object.messageType ?? "";
    return message;
  },
};

function createBaseEmpty(): Empty {
  return {};
}

export const Empty = {
  encode(_: Empty, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): Empty {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseEmpty();
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
  // Transform<Empty, Uint8Array>
  async *encodeTransform(
    source: AsyncIterable<Empty | Empty[]> | Iterable<Empty | Empty[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [Empty.encode(p).finish()];
        }
      } else {
        yield* [Empty.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, Empty>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<Empty> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [Empty.decode(p)];
        }
      } else {
        yield* [Empty.decode(pkt as any)];
      }
    }
  },

  fromJSON(_: any): Empty {
    return {};
  },

  toJSON(_: Empty): unknown {
    const obj: any = {};
    return obj;
  },

  create(base?: DeepPartial<Empty>): Empty {
    return Empty.fromPartial(base ?? {});
  },
  fromPartial(_: DeepPartial<Empty>): Empty {
    const message = createBaseEmpty();
    return message;
  },
};

export type NetworkMessageServiceService = typeof NetworkMessageServiceService;
export const NetworkMessageServiceService = {
  simpleMsgExchange: {
    path: "/velor.remote_executor.v1.NetworkMessageService/SimpleMsgExchange",
    requestStream: false,
    responseStream: false,
    requestSerialize: (value: NetworkMessage) => Buffer.from(NetworkMessage.encode(value).finish()),
    requestDeserialize: (value: Buffer) => NetworkMessage.decode(value),
    responseSerialize: (value: Empty) => Buffer.from(Empty.encode(value).finish()),
    responseDeserialize: (value: Buffer) => Empty.decode(value),
  },
} as const;

export interface NetworkMessageServiceServer extends UntypedServiceImplementation {
  simpleMsgExchange: handleUnaryCall<NetworkMessage, Empty>;
}

export interface NetworkMessageServiceClient extends Client {
  simpleMsgExchange(
    request: NetworkMessage,
    callback: (error: ServiceError | null, response: Empty) => void,
  ): ClientUnaryCall;
  simpleMsgExchange(
    request: NetworkMessage,
    metadata: Metadata,
    callback: (error: ServiceError | null, response: Empty) => void,
  ): ClientUnaryCall;
  simpleMsgExchange(
    request: NetworkMessage,
    metadata: Metadata,
    options: Partial<CallOptions>,
    callback: (error: ServiceError | null, response: Empty) => void,
  ): ClientUnaryCall;
}

export const NetworkMessageServiceClient = makeGenericClientConstructor(
  NetworkMessageServiceService,
  "velor.remote_executor.v1.NetworkMessageService",
) as unknown as {
  new (address: string, credentials: ChannelCredentials, options?: Partial<ClientOptions>): NetworkMessageServiceClient;
  service: typeof NetworkMessageServiceService;
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

function isSet(value: any): boolean {
  return value !== null && value !== undefined;
}
