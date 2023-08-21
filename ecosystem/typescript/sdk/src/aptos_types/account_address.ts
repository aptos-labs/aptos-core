// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

import { HexString, MaybeHexString } from "../utils";
import { Serializer, Deserializer, Bytes } from "../bcs";

/**
 * Exported as TransactionBuilderTypes.AccountAddress
 */
export class AccountAddress {
  static readonly LENGTH: number = 32;

  readonly address: Bytes;

  static CORE_CODE_ADDRESS: AccountAddress = AccountAddress.fromHex("0x1");

  constructor(address: Bytes) {
    if (address.length !== AccountAddress.LENGTH) {
      throw new Error("Expected address of length 32");
    }
    this.address = address;
  }

  /**
   * Creates AccountAddress from a hex string.
   * @param addr Hex string can be with a prefix or without a prefix,
   *   e.g. '0x1aa' or '1aa'. Hex string will be left padded with 0s if too short.
   */
  static fromHex(addr: MaybeHexString): AccountAddress {
    let address = HexString.ensure(addr);

    // If an address hex has odd number of digits, padd the hex string with 0
    // e.g. '1aa' would become '01aa'.
    if (address.noPrefix().length % 2 !== 0) {
      address = new HexString(`0${address.noPrefix()}`);
    }

    const addressBytes = address.toUint8Array();

    if (addressBytes.length > AccountAddress.LENGTH) {
      // eslint-disable-next-line quotes
      throw new Error("Hex string is too long. Address's length is 32 bytes.");
    } else if (addressBytes.length === AccountAddress.LENGTH) {
      return new AccountAddress(addressBytes);
    }

    const res: Bytes = new Uint8Array(AccountAddress.LENGTH);
    res.set(addressBytes, AccountAddress.LENGTH - addressBytes.length);

    return new AccountAddress(res);
  }

  /**
   * Checks if the string is a valid AccountAddress
   * @param addr Hex string can be with a prefix or without a prefix,
   *   e.g. '0x1aa' or '1aa'. Hex string will be left padded with 0s if too short.
   */
  static isValid(addr: MaybeHexString): boolean {
    // At least one zero is required
    if (addr === "") {
      return false;
    }

    let address = HexString.ensure(addr);

    // If an address hex has odd number of digits, padd the hex string with 0
    // e.g. '1aa' would become '01aa'.
    if (address.noPrefix().length % 2 !== 0) {
      address = new HexString(`0${address.noPrefix()}`);
    }

    const addressBytes = address.toUint8Array();

    return addressBytes.length <= AccountAddress.LENGTH;
  }

  /**
   * Return a hex string from account Address.
   */
  toHexString(): MaybeHexString {
    return HexString.fromUint8Array(this.address).hex();
  }

  serialize(serializer: Serializer): void {
    serializer.serializeFixedBytes(this.address);
  }

  static deserialize(deserializer: Deserializer): AccountAddress {
    return new AccountAddress(deserializer.deserializeFixedBytes(AccountAddress.LENGTH));
  }

  /**
   * Standardizes an address to the format "0x" followed by 64 lowercase hexadecimal digits.
   */
  static standardizeAddress(address: string): string {
    // Convert the address to lowercase
    const lowercaseAddress = address.toLowerCase();
    // Remove the "0x" prefix if present
    const addressWithoutPrefix = lowercaseAddress.startsWith("0x") ? lowercaseAddress.slice(2) : lowercaseAddress;
    // Pad the address with leading zeros if necessary
    // to ensure it has exactly 64 characters (excluding the "0x" prefix)
    const addressWithPadding = addressWithoutPrefix.padStart(64, "0");
    // Return the standardized address with the "0x" prefix
    return `0x${addressWithPadding}`;
  }
}
