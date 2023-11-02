/* eslint-disable */
import Long from "long";
import _m0 from "protobufjs/minimal";

export interface Timestamp {
  /**
   * Represents seconds of UTC time since Unix epoch
   * 1970-01-01T00:00:00Z. Must be from 0001-01-01T00:00:00Z to
   * 9999-12-31T23:59:59Z inclusive.
   */
  seconds?:
    | bigint
    | undefined;
  /**
   * Non-negative fractions of a second at nanosecond resolution. Negative
   * second values with fractions must still have non-negative nanos values
   * that count forward in time. Must be from 0 to 999,999,999
   * inclusive.
   */
  nanos?: number | undefined;
}

function createBaseTimestamp(): Timestamp {
  return { seconds: BigInt("0"), nanos: 0 };
}

export const Timestamp = {
  encode(message: Timestamp, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.seconds !== undefined && message.seconds !== BigInt("0")) {
      if (BigInt.asIntN(64, message.seconds) !== message.seconds) {
        throw new Error("value provided for field message.seconds of type int64 too large");
      }
      writer.uint32(8).int64(message.seconds.toString());
    }
    if (message.nanos !== undefined && message.nanos !== 0) {
      writer.uint32(16).int32(message.nanos);
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): Timestamp {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseTimestamp();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 8) {
            break;
          }

          message.seconds = longToBigint(reader.int64() as Long);
          continue;
        case 2:
          if (tag !== 16) {
            break;
          }

          message.nanos = reader.int32();
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
  // Transform<Timestamp, Uint8Array>
  async *encodeTransform(
    source: AsyncIterable<Timestamp | Timestamp[]> | Iterable<Timestamp | Timestamp[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [Timestamp.encode(p).finish()];
        }
      } else {
        yield* [Timestamp.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, Timestamp>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<Timestamp> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [Timestamp.decode(p)];
        }
      } else {
        yield* [Timestamp.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): Timestamp {
    return {
      seconds: isSet(object.seconds) ? BigInt(object.seconds) : BigInt("0"),
      nanos: isSet(object.nanos) ? globalThis.Number(object.nanos) : 0,
    };
  },

  toJSON(message: Timestamp): unknown {
    const obj: any = {};
    if (message.seconds !== undefined && message.seconds !== BigInt("0")) {
      obj.seconds = message.seconds.toString();
    }
    if (message.nanos !== undefined && message.nanos !== 0) {
      obj.nanos = Math.round(message.nanos);
    }
    return obj;
  },

  create(base?: DeepPartial<Timestamp>): Timestamp {
    return Timestamp.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<Timestamp>): Timestamp {
    const message = createBaseTimestamp();
    message.seconds = object.seconds ?? BigInt("0");
    message.nanos = object.nanos ?? 0;
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
