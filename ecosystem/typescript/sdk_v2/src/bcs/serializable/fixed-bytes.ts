// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

import { Serializable, Serializer } from "../serializer";
import { Deserializer } from "../deserializer";
import { HexInput } from "../../types";
import { Hex } from "../../core";

/**
 *  This class exists to represent a contiguous sequence of BCS bytes that when serialized
 *  do *not* prepend the length of the byte sequence at the beginning.
 *
 *  The main time to use this class is when you are passing around already BCS-serialized bytes
 *  that do not need to undergo another round of BCS serialization.
 *
 *  For example, if you store each of the 32 bytes for an address as a U8 in a MoveVector<U8>, when you
 *  serialize that MoveVector<U8>, it will be serialized to 33 bytes. If you solely want to pass around
 *  the 32 bytes as a Serializable class that *does not* prepend the length to the BCS-serialized representation,
 *  use this class.
 *
 * @params value: HexInput representing a sequence of Uint8 bytes
 * @returns a Serializable FixedBytes instance, which when serialized, does not prepend the length of the bytes
 * @example
 * const address = AccountAddress.ONE;
 * const bytes = address.bcsToBytes();
 * // bytes is the Move serialized version of an address
 * // it has a fixed length, meaning it doesn't have a length at the beginning.
 * const fixedBytes = new FixedBytes(bytes);
 * // or, say, deserializing it from a sequence of bytes and you *do* know the length
 * const fixedBytes = FixedBytes.deserialize(deserializer, 32);
 * @see EntryFunction
 */
export class FixedBytes extends Serializable {
  public value: Uint8Array;

  constructor(value: HexInput) {
    super();
    this.value = Hex.fromHexInput({ hexInput: value }).toUint8Array();
  }

  serialize(serializer: Serializer): void {
    serializer.serializeFixedBytes(this.value);
  }

  static deserialize(deserializer: Deserializer, length: number): FixedBytes {
    const bytes = deserializer.deserializeFixedBytes(length);
    return new FixedBytes(bytes);
  }
}
