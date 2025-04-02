/* eslint-disable */
import _m0 from "protobufjs/minimal";
import {
  Transaction_TransactionType,
  transaction_TransactionTypeFromJSON,
  transaction_TransactionTypeToJSON,
} from "../../transaction/v1/transaction";

export interface LogicalAndFilters {
  filters?: BooleanTransactionFilter[] | undefined;
}

export interface LogicalOrFilters {
  filters?: BooleanTransactionFilter[] | undefined;
}

export interface TransactionRootFilter {
  success?: boolean | undefined;
  transactionType?: Transaction_TransactionType | undefined;
}

export interface EntryFunctionFilter {
  address?: string | undefined;
  moduleName?: string | undefined;
  function?: string | undefined;
}

export interface UserTransactionPayloadFilter {
  entryFunctionFilter?: EntryFunctionFilter | undefined;
}

export interface UserTransactionFilter {
  sender?: string | undefined;
  payloadFilter?: UserTransactionPayloadFilter | undefined;
}

export interface MoveStructTagFilter {
  address?: string | undefined;
  module?: string | undefined;
  name?: string | undefined;
}

export interface EventFilter {
  structType?: MoveStructTagFilter | undefined;
  dataSubstringFilter?: string | undefined;
}

export interface APIFilter {
  transactionRootFilter?: TransactionRootFilter | undefined;
  userTransactionFilter?: UserTransactionFilter | undefined;
  eventFilter?: EventFilter | undefined;
}

export interface BooleanTransactionFilter {
  apiFilter?: APIFilter | undefined;
  logicalAnd?: LogicalAndFilters | undefined;
  logicalOr?: LogicalOrFilters | undefined;
  logicalNot?: BooleanTransactionFilter | undefined;
}

function createBaseLogicalAndFilters(): LogicalAndFilters {
  return { filters: [] };
}

export const LogicalAndFilters = {
  encode(message: LogicalAndFilters, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.filters !== undefined && message.filters.length !== 0) {
      for (const v of message.filters) {
        BooleanTransactionFilter.encode(v!, writer.uint32(10).fork()).ldelim();
      }
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): LogicalAndFilters {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseLogicalAndFilters();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.filters!.push(BooleanTransactionFilter.decode(reader, reader.uint32()));
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
  // Transform<LogicalAndFilters, Uint8Array>
  async *encodeTransform(
    source: AsyncIterable<LogicalAndFilters | LogicalAndFilters[]> | Iterable<LogicalAndFilters | LogicalAndFilters[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [LogicalAndFilters.encode(p).finish()];
        }
      } else {
        yield* [LogicalAndFilters.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, LogicalAndFilters>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<LogicalAndFilters> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [LogicalAndFilters.decode(p)];
        }
      } else {
        yield* [LogicalAndFilters.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): LogicalAndFilters {
    return {
      filters: globalThis.Array.isArray(object?.filters)
        ? object.filters.map((e: any) => BooleanTransactionFilter.fromJSON(e))
        : [],
    };
  },

  toJSON(message: LogicalAndFilters): unknown {
    const obj: any = {};
    if (message.filters?.length) {
      obj.filters = message.filters.map((e) => BooleanTransactionFilter.toJSON(e));
    }
    return obj;
  },

  create(base?: DeepPartial<LogicalAndFilters>): LogicalAndFilters {
    return LogicalAndFilters.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<LogicalAndFilters>): LogicalAndFilters {
    const message = createBaseLogicalAndFilters();
    message.filters = object.filters?.map((e) => BooleanTransactionFilter.fromPartial(e)) || [];
    return message;
  },
};

function createBaseLogicalOrFilters(): LogicalOrFilters {
  return { filters: [] };
}

export const LogicalOrFilters = {
  encode(message: LogicalOrFilters, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.filters !== undefined && message.filters.length !== 0) {
      for (const v of message.filters) {
        BooleanTransactionFilter.encode(v!, writer.uint32(10).fork()).ldelim();
      }
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): LogicalOrFilters {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseLogicalOrFilters();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.filters!.push(BooleanTransactionFilter.decode(reader, reader.uint32()));
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
  // Transform<LogicalOrFilters, Uint8Array>
  async *encodeTransform(
    source: AsyncIterable<LogicalOrFilters | LogicalOrFilters[]> | Iterable<LogicalOrFilters | LogicalOrFilters[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [LogicalOrFilters.encode(p).finish()];
        }
      } else {
        yield* [LogicalOrFilters.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, LogicalOrFilters>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<LogicalOrFilters> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [LogicalOrFilters.decode(p)];
        }
      } else {
        yield* [LogicalOrFilters.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): LogicalOrFilters {
    return {
      filters: globalThis.Array.isArray(object?.filters)
        ? object.filters.map((e: any) => BooleanTransactionFilter.fromJSON(e))
        : [],
    };
  },

  toJSON(message: LogicalOrFilters): unknown {
    const obj: any = {};
    if (message.filters?.length) {
      obj.filters = message.filters.map((e) => BooleanTransactionFilter.toJSON(e));
    }
    return obj;
  },

  create(base?: DeepPartial<LogicalOrFilters>): LogicalOrFilters {
    return LogicalOrFilters.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<LogicalOrFilters>): LogicalOrFilters {
    const message = createBaseLogicalOrFilters();
    message.filters = object.filters?.map((e) => BooleanTransactionFilter.fromPartial(e)) || [];
    return message;
  },
};

function createBaseTransactionRootFilter(): TransactionRootFilter {
  return { success: undefined, transactionType: undefined };
}

export const TransactionRootFilter = {
  encode(message: TransactionRootFilter, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.success !== undefined) {
      writer.uint32(8).bool(message.success);
    }
    if (message.transactionType !== undefined) {
      writer.uint32(16).int32(message.transactionType);
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): TransactionRootFilter {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseTransactionRootFilter();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 8) {
            break;
          }

          message.success = reader.bool();
          continue;
        case 2:
          if (tag !== 16) {
            break;
          }

          message.transactionType = reader.int32() as any;
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
  // Transform<TransactionRootFilter, Uint8Array>
  async *encodeTransform(
    source:
      | AsyncIterable<TransactionRootFilter | TransactionRootFilter[]>
      | Iterable<TransactionRootFilter | TransactionRootFilter[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [TransactionRootFilter.encode(p).finish()];
        }
      } else {
        yield* [TransactionRootFilter.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, TransactionRootFilter>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<TransactionRootFilter> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [TransactionRootFilter.decode(p)];
        }
      } else {
        yield* [TransactionRootFilter.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): TransactionRootFilter {
    return {
      success: isSet(object.success) ? globalThis.Boolean(object.success) : undefined,
      transactionType: isSet(object.transactionType)
        ? transaction_TransactionTypeFromJSON(object.transactionType)
        : undefined,
    };
  },

  toJSON(message: TransactionRootFilter): unknown {
    const obj: any = {};
    if (message.success !== undefined) {
      obj.success = message.success;
    }
    if (message.transactionType !== undefined) {
      obj.transactionType = transaction_TransactionTypeToJSON(message.transactionType);
    }
    return obj;
  },

  create(base?: DeepPartial<TransactionRootFilter>): TransactionRootFilter {
    return TransactionRootFilter.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<TransactionRootFilter>): TransactionRootFilter {
    const message = createBaseTransactionRootFilter();
    message.success = object.success ?? undefined;
    message.transactionType = object.transactionType ?? undefined;
    return message;
  },
};

function createBaseEntryFunctionFilter(): EntryFunctionFilter {
  return { address: undefined, moduleName: undefined, function: undefined };
}

export const EntryFunctionFilter = {
  encode(message: EntryFunctionFilter, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.address !== undefined) {
      writer.uint32(10).string(message.address);
    }
    if (message.moduleName !== undefined) {
      writer.uint32(18).string(message.moduleName);
    }
    if (message.function !== undefined) {
      writer.uint32(26).string(message.function);
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): EntryFunctionFilter {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseEntryFunctionFilter();
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

          message.moduleName = reader.string();
          continue;
        case 3:
          if (tag !== 26) {
            break;
          }

          message.function = reader.string();
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
  // Transform<EntryFunctionFilter, Uint8Array>
  async *encodeTransform(
    source:
      | AsyncIterable<EntryFunctionFilter | EntryFunctionFilter[]>
      | Iterable<EntryFunctionFilter | EntryFunctionFilter[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [EntryFunctionFilter.encode(p).finish()];
        }
      } else {
        yield* [EntryFunctionFilter.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, EntryFunctionFilter>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<EntryFunctionFilter> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [EntryFunctionFilter.decode(p)];
        }
      } else {
        yield* [EntryFunctionFilter.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): EntryFunctionFilter {
    return {
      address: isSet(object.address) ? globalThis.String(object.address) : undefined,
      moduleName: isSet(object.moduleName) ? globalThis.String(object.moduleName) : undefined,
      function: isSet(object.function) ? globalThis.String(object.function) : undefined,
    };
  },

  toJSON(message: EntryFunctionFilter): unknown {
    const obj: any = {};
    if (message.address !== undefined) {
      obj.address = message.address;
    }
    if (message.moduleName !== undefined) {
      obj.moduleName = message.moduleName;
    }
    if (message.function !== undefined) {
      obj.function = message.function;
    }
    return obj;
  },

  create(base?: DeepPartial<EntryFunctionFilter>): EntryFunctionFilter {
    return EntryFunctionFilter.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<EntryFunctionFilter>): EntryFunctionFilter {
    const message = createBaseEntryFunctionFilter();
    message.address = object.address ?? undefined;
    message.moduleName = object.moduleName ?? undefined;
    message.function = object.function ?? undefined;
    return message;
  },
};

function createBaseUserTransactionPayloadFilter(): UserTransactionPayloadFilter {
  return { entryFunctionFilter: undefined };
}

export const UserTransactionPayloadFilter = {
  encode(message: UserTransactionPayloadFilter, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.entryFunctionFilter !== undefined) {
      EntryFunctionFilter.encode(message.entryFunctionFilter, writer.uint32(10).fork()).ldelim();
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): UserTransactionPayloadFilter {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseUserTransactionPayloadFilter();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.entryFunctionFilter = EntryFunctionFilter.decode(reader, reader.uint32());
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
  // Transform<UserTransactionPayloadFilter, Uint8Array>
  async *encodeTransform(
    source:
      | AsyncIterable<UserTransactionPayloadFilter | UserTransactionPayloadFilter[]>
      | Iterable<UserTransactionPayloadFilter | UserTransactionPayloadFilter[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [UserTransactionPayloadFilter.encode(p).finish()];
        }
      } else {
        yield* [UserTransactionPayloadFilter.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, UserTransactionPayloadFilter>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<UserTransactionPayloadFilter> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [UserTransactionPayloadFilter.decode(p)];
        }
      } else {
        yield* [UserTransactionPayloadFilter.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): UserTransactionPayloadFilter {
    return {
      entryFunctionFilter: isSet(object.entryFunctionFilter)
        ? EntryFunctionFilter.fromJSON(object.entryFunctionFilter)
        : undefined,
    };
  },

  toJSON(message: UserTransactionPayloadFilter): unknown {
    const obj: any = {};
    if (message.entryFunctionFilter !== undefined) {
      obj.entryFunctionFilter = EntryFunctionFilter.toJSON(message.entryFunctionFilter);
    }
    return obj;
  },

  create(base?: DeepPartial<UserTransactionPayloadFilter>): UserTransactionPayloadFilter {
    return UserTransactionPayloadFilter.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<UserTransactionPayloadFilter>): UserTransactionPayloadFilter {
    const message = createBaseUserTransactionPayloadFilter();
    message.entryFunctionFilter = (object.entryFunctionFilter !== undefined && object.entryFunctionFilter !== null)
      ? EntryFunctionFilter.fromPartial(object.entryFunctionFilter)
      : undefined;
    return message;
  },
};

function createBaseUserTransactionFilter(): UserTransactionFilter {
  return { sender: undefined, payloadFilter: undefined };
}

export const UserTransactionFilter = {
  encode(message: UserTransactionFilter, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.sender !== undefined) {
      writer.uint32(10).string(message.sender);
    }
    if (message.payloadFilter !== undefined) {
      UserTransactionPayloadFilter.encode(message.payloadFilter, writer.uint32(18).fork()).ldelim();
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): UserTransactionFilter {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseUserTransactionFilter();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.sender = reader.string();
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.payloadFilter = UserTransactionPayloadFilter.decode(reader, reader.uint32());
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
  // Transform<UserTransactionFilter, Uint8Array>
  async *encodeTransform(
    source:
      | AsyncIterable<UserTransactionFilter | UserTransactionFilter[]>
      | Iterable<UserTransactionFilter | UserTransactionFilter[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [UserTransactionFilter.encode(p).finish()];
        }
      } else {
        yield* [UserTransactionFilter.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, UserTransactionFilter>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<UserTransactionFilter> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [UserTransactionFilter.decode(p)];
        }
      } else {
        yield* [UserTransactionFilter.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): UserTransactionFilter {
    return {
      sender: isSet(object.sender) ? globalThis.String(object.sender) : undefined,
      payloadFilter: isSet(object.payloadFilter)
        ? UserTransactionPayloadFilter.fromJSON(object.payloadFilter)
        : undefined,
    };
  },

  toJSON(message: UserTransactionFilter): unknown {
    const obj: any = {};
    if (message.sender !== undefined) {
      obj.sender = message.sender;
    }
    if (message.payloadFilter !== undefined) {
      obj.payloadFilter = UserTransactionPayloadFilter.toJSON(message.payloadFilter);
    }
    return obj;
  },

  create(base?: DeepPartial<UserTransactionFilter>): UserTransactionFilter {
    return UserTransactionFilter.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<UserTransactionFilter>): UserTransactionFilter {
    const message = createBaseUserTransactionFilter();
    message.sender = object.sender ?? undefined;
    message.payloadFilter = (object.payloadFilter !== undefined && object.payloadFilter !== null)
      ? UserTransactionPayloadFilter.fromPartial(object.payloadFilter)
      : undefined;
    return message;
  },
};

function createBaseMoveStructTagFilter(): MoveStructTagFilter {
  return { address: undefined, module: undefined, name: undefined };
}

export const MoveStructTagFilter = {
  encode(message: MoveStructTagFilter, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.address !== undefined) {
      writer.uint32(10).string(message.address);
    }
    if (message.module !== undefined) {
      writer.uint32(18).string(message.module);
    }
    if (message.name !== undefined) {
      writer.uint32(26).string(message.name);
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): MoveStructTagFilter {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseMoveStructTagFilter();
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

          message.module = reader.string();
          continue;
        case 3:
          if (tag !== 26) {
            break;
          }

          message.name = reader.string();
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
  // Transform<MoveStructTagFilter, Uint8Array>
  async *encodeTransform(
    source:
      | AsyncIterable<MoveStructTagFilter | MoveStructTagFilter[]>
      | Iterable<MoveStructTagFilter | MoveStructTagFilter[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [MoveStructTagFilter.encode(p).finish()];
        }
      } else {
        yield* [MoveStructTagFilter.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, MoveStructTagFilter>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<MoveStructTagFilter> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [MoveStructTagFilter.decode(p)];
        }
      } else {
        yield* [MoveStructTagFilter.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): MoveStructTagFilter {
    return {
      address: isSet(object.address) ? globalThis.String(object.address) : undefined,
      module: isSet(object.module) ? globalThis.String(object.module) : undefined,
      name: isSet(object.name) ? globalThis.String(object.name) : undefined,
    };
  },

  toJSON(message: MoveStructTagFilter): unknown {
    const obj: any = {};
    if (message.address !== undefined) {
      obj.address = message.address;
    }
    if (message.module !== undefined) {
      obj.module = message.module;
    }
    if (message.name !== undefined) {
      obj.name = message.name;
    }
    return obj;
  },

  create(base?: DeepPartial<MoveStructTagFilter>): MoveStructTagFilter {
    return MoveStructTagFilter.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<MoveStructTagFilter>): MoveStructTagFilter {
    const message = createBaseMoveStructTagFilter();
    message.address = object.address ?? undefined;
    message.module = object.module ?? undefined;
    message.name = object.name ?? undefined;
    return message;
  },
};

function createBaseEventFilter(): EventFilter {
  return { structType: undefined, dataSubstringFilter: undefined };
}

export const EventFilter = {
  encode(message: EventFilter, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.structType !== undefined) {
      MoveStructTagFilter.encode(message.structType, writer.uint32(10).fork()).ldelim();
    }
    if (message.dataSubstringFilter !== undefined) {
      writer.uint32(18).string(message.dataSubstringFilter);
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): EventFilter {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseEventFilter();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.structType = MoveStructTagFilter.decode(reader, reader.uint32());
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.dataSubstringFilter = reader.string();
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
  // Transform<EventFilter, Uint8Array>
  async *encodeTransform(
    source: AsyncIterable<EventFilter | EventFilter[]> | Iterable<EventFilter | EventFilter[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [EventFilter.encode(p).finish()];
        }
      } else {
        yield* [EventFilter.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, EventFilter>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<EventFilter> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [EventFilter.decode(p)];
        }
      } else {
        yield* [EventFilter.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): EventFilter {
    return {
      structType: isSet(object.structType) ? MoveStructTagFilter.fromJSON(object.structType) : undefined,
      dataSubstringFilter: isSet(object.dataSubstringFilter)
        ? globalThis.String(object.dataSubstringFilter)
        : undefined,
    };
  },

  toJSON(message: EventFilter): unknown {
    const obj: any = {};
    if (message.structType !== undefined) {
      obj.structType = MoveStructTagFilter.toJSON(message.structType);
    }
    if (message.dataSubstringFilter !== undefined) {
      obj.dataSubstringFilter = message.dataSubstringFilter;
    }
    return obj;
  },

  create(base?: DeepPartial<EventFilter>): EventFilter {
    return EventFilter.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<EventFilter>): EventFilter {
    const message = createBaseEventFilter();
    message.structType = (object.structType !== undefined && object.structType !== null)
      ? MoveStructTagFilter.fromPartial(object.structType)
      : undefined;
    message.dataSubstringFilter = object.dataSubstringFilter ?? undefined;
    return message;
  },
};

function createBaseAPIFilter(): APIFilter {
  return { transactionRootFilter: undefined, userTransactionFilter: undefined, eventFilter: undefined };
}

export const APIFilter = {
  encode(message: APIFilter, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.transactionRootFilter !== undefined) {
      TransactionRootFilter.encode(message.transactionRootFilter, writer.uint32(10).fork()).ldelim();
    }
    if (message.userTransactionFilter !== undefined) {
      UserTransactionFilter.encode(message.userTransactionFilter, writer.uint32(18).fork()).ldelim();
    }
    if (message.eventFilter !== undefined) {
      EventFilter.encode(message.eventFilter, writer.uint32(26).fork()).ldelim();
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): APIFilter {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseAPIFilter();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.transactionRootFilter = TransactionRootFilter.decode(reader, reader.uint32());
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.userTransactionFilter = UserTransactionFilter.decode(reader, reader.uint32());
          continue;
        case 3:
          if (tag !== 26) {
            break;
          }

          message.eventFilter = EventFilter.decode(reader, reader.uint32());
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
  // Transform<APIFilter, Uint8Array>
  async *encodeTransform(
    source: AsyncIterable<APIFilter | APIFilter[]> | Iterable<APIFilter | APIFilter[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [APIFilter.encode(p).finish()];
        }
      } else {
        yield* [APIFilter.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, APIFilter>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<APIFilter> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [APIFilter.decode(p)];
        }
      } else {
        yield* [APIFilter.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): APIFilter {
    return {
      transactionRootFilter: isSet(object.transactionRootFilter)
        ? TransactionRootFilter.fromJSON(object.transactionRootFilter)
        : undefined,
      userTransactionFilter: isSet(object.userTransactionFilter)
        ? UserTransactionFilter.fromJSON(object.userTransactionFilter)
        : undefined,
      eventFilter: isSet(object.eventFilter) ? EventFilter.fromJSON(object.eventFilter) : undefined,
    };
  },

  toJSON(message: APIFilter): unknown {
    const obj: any = {};
    if (message.transactionRootFilter !== undefined) {
      obj.transactionRootFilter = TransactionRootFilter.toJSON(message.transactionRootFilter);
    }
    if (message.userTransactionFilter !== undefined) {
      obj.userTransactionFilter = UserTransactionFilter.toJSON(message.userTransactionFilter);
    }
    if (message.eventFilter !== undefined) {
      obj.eventFilter = EventFilter.toJSON(message.eventFilter);
    }
    return obj;
  },

  create(base?: DeepPartial<APIFilter>): APIFilter {
    return APIFilter.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<APIFilter>): APIFilter {
    const message = createBaseAPIFilter();
    message.transactionRootFilter =
      (object.transactionRootFilter !== undefined && object.transactionRootFilter !== null)
        ? TransactionRootFilter.fromPartial(object.transactionRootFilter)
        : undefined;
    message.userTransactionFilter =
      (object.userTransactionFilter !== undefined && object.userTransactionFilter !== null)
        ? UserTransactionFilter.fromPartial(object.userTransactionFilter)
        : undefined;
    message.eventFilter = (object.eventFilter !== undefined && object.eventFilter !== null)
      ? EventFilter.fromPartial(object.eventFilter)
      : undefined;
    return message;
  },
};

function createBaseBooleanTransactionFilter(): BooleanTransactionFilter {
  return { apiFilter: undefined, logicalAnd: undefined, logicalOr: undefined, logicalNot: undefined };
}

export const BooleanTransactionFilter = {
  encode(message: BooleanTransactionFilter, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.apiFilter !== undefined) {
      APIFilter.encode(message.apiFilter, writer.uint32(10).fork()).ldelim();
    }
    if (message.logicalAnd !== undefined) {
      LogicalAndFilters.encode(message.logicalAnd, writer.uint32(18).fork()).ldelim();
    }
    if (message.logicalOr !== undefined) {
      LogicalOrFilters.encode(message.logicalOr, writer.uint32(26).fork()).ldelim();
    }
    if (message.logicalNot !== undefined) {
      BooleanTransactionFilter.encode(message.logicalNot, writer.uint32(34).fork()).ldelim();
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): BooleanTransactionFilter {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseBooleanTransactionFilter();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.apiFilter = APIFilter.decode(reader, reader.uint32());
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.logicalAnd = LogicalAndFilters.decode(reader, reader.uint32());
          continue;
        case 3:
          if (tag !== 26) {
            break;
          }

          message.logicalOr = LogicalOrFilters.decode(reader, reader.uint32());
          continue;
        case 4:
          if (tag !== 34) {
            break;
          }

          message.logicalNot = BooleanTransactionFilter.decode(reader, reader.uint32());
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
  // Transform<BooleanTransactionFilter, Uint8Array>
  async *encodeTransform(
    source:
      | AsyncIterable<BooleanTransactionFilter | BooleanTransactionFilter[]>
      | Iterable<BooleanTransactionFilter | BooleanTransactionFilter[]>,
  ): AsyncIterable<Uint8Array> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [BooleanTransactionFilter.encode(p).finish()];
        }
      } else {
        yield* [BooleanTransactionFilter.encode(pkt as any).finish()];
      }
    }
  },

  // decodeTransform decodes a source of encoded messages.
  // Transform<Uint8Array, BooleanTransactionFilter>
  async *decodeTransform(
    source: AsyncIterable<Uint8Array | Uint8Array[]> | Iterable<Uint8Array | Uint8Array[]>,
  ): AsyncIterable<BooleanTransactionFilter> {
    for await (const pkt of source) {
      if (globalThis.Array.isArray(pkt)) {
        for (const p of (pkt as any)) {
          yield* [BooleanTransactionFilter.decode(p)];
        }
      } else {
        yield* [BooleanTransactionFilter.decode(pkt as any)];
      }
    }
  },

  fromJSON(object: any): BooleanTransactionFilter {
    return {
      apiFilter: isSet(object.apiFilter) ? APIFilter.fromJSON(object.apiFilter) : undefined,
      logicalAnd: isSet(object.logicalAnd) ? LogicalAndFilters.fromJSON(object.logicalAnd) : undefined,
      logicalOr: isSet(object.logicalOr) ? LogicalOrFilters.fromJSON(object.logicalOr) : undefined,
      logicalNot: isSet(object.logicalNot) ? BooleanTransactionFilter.fromJSON(object.logicalNot) : undefined,
    };
  },

  toJSON(message: BooleanTransactionFilter): unknown {
    const obj: any = {};
    if (message.apiFilter !== undefined) {
      obj.apiFilter = APIFilter.toJSON(message.apiFilter);
    }
    if (message.logicalAnd !== undefined) {
      obj.logicalAnd = LogicalAndFilters.toJSON(message.logicalAnd);
    }
    if (message.logicalOr !== undefined) {
      obj.logicalOr = LogicalOrFilters.toJSON(message.logicalOr);
    }
    if (message.logicalNot !== undefined) {
      obj.logicalNot = BooleanTransactionFilter.toJSON(message.logicalNot);
    }
    return obj;
  },

  create(base?: DeepPartial<BooleanTransactionFilter>): BooleanTransactionFilter {
    return BooleanTransactionFilter.fromPartial(base ?? {});
  },
  fromPartial(object: DeepPartial<BooleanTransactionFilter>): BooleanTransactionFilter {
    const message = createBaseBooleanTransactionFilter();
    message.apiFilter = (object.apiFilter !== undefined && object.apiFilter !== null)
      ? APIFilter.fromPartial(object.apiFilter)
      : undefined;
    message.logicalAnd = (object.logicalAnd !== undefined && object.logicalAnd !== null)
      ? LogicalAndFilters.fromPartial(object.logicalAnd)
      : undefined;
    message.logicalOr = (object.logicalOr !== undefined && object.logicalOr !== null)
      ? LogicalOrFilters.fromPartial(object.logicalOr)
      : undefined;
    message.logicalNot = (object.logicalNot !== undefined && object.logicalNot !== null)
      ? BooleanTransactionFilter.fromPartial(object.logicalNot)
      : undefined;
    return message;
  },
};

type Builtin = Date | Function | Uint8Array | string | number | boolean | bigint | undefined;

type DeepPartial<T> = T extends Builtin ? T
  : T extends globalThis.Array<infer U> ? globalThis.Array<DeepPartial<U>>
  : T extends ReadonlyArray<infer U> ? ReadonlyArray<DeepPartial<U>>
  : T extends {} ? { [K in keyof T]?: DeepPartial<T[K]> }
  : Partial<T>;

function isSet(value: any): boolean {
  return value !== null && value !== undefined;
}
