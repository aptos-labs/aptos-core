// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

import { bytesToHex, hexToBytes } from "@noble/hashes/utils";
import { HexInput } from "../types";
import { ParsingError, ParsingResult } from "./common";

/**
 * This enum is used to explain why an address was invalid.
 */
export enum AddressInvalidReason {
  INCORRECT_NUMBER_OF_BYTES = "incorrect_number_of_bytes",
  INVALID_HEX_CHARS = "invalid_hex_chars",
  TOO_SHORT = "too_short",
  TOO_LONG = "too_long",
  LEADING_ZERO_X_REQUIRED = "leading_zero_x_required",
  LONG_FORM_REQUIRED_UNLESS_SPECIAL = "long_form_required_unless_special",
  INVALID_PADDING_ZEROES = "INVALID_PADDING_ZEROES",
}

/**
 * NOTE: Only use this class for account addresses. For other hex data, e.g. transaction
 * hashes, use the Hex class.
 *
 * AccountAddress is used for working with account addresses. Account addresses, when
 * represented as a string, generally look like these examples:
 * - 0x1
 * - 0xaa86fe99004361f747f91342ca13c426ca0cccb0c1217677180c9493bad6ef0c
 *
 * Proper formatting and parsing of account addresses is defined by AIP-40.
 * To learn more about the standard, read the AIP here:
 * https://github.com/aptos-foundation/AIPs/blob/main/aips/aip-40.md.
 *
 * The comments in this class make frequent reference to the LONG and SHORT formats,
 * as well as "special" addresses. To learn what these refer to see AIP-40.
 */
export class AccountAddress {
  /*
   * This is the internal representation of an account address.
   */
  readonly data: Uint8Array;

  /*
   * The number of bytes that make up an account address.
   */
  static readonly LENGTH: number = 32;

  /*
   * The length of an address string in LONG form without a leading 0x.
   */
  static readonly LONG_STRING_LENGTH: number = 64;

  static ONE: AccountAddress = AccountAddress.fromString({ input: "0x1" });

  static TWO: AccountAddress = AccountAddress.fromString({ input: "0x2" });

  static THREE: AccountAddress = AccountAddress.fromString({ input: "0x3" });

  static FOUR: AccountAddress = AccountAddress.fromString({ input: "0x4" });

  /**
   * Creates an instance of AccountAddress from a Uint8Array.
   *
   * @param args.data A Uint8Array representing an account address.
   */
  constructor(args: { data: Uint8Array }) {
    if (args.data.length !== AccountAddress.LENGTH) {
      throw new ParsingError(
        "AccountAddress data should be exactly 32 bytes long",
        AddressInvalidReason.INCORRECT_NUMBER_OF_BYTES,
      );
    }
    this.data = args.data;
  }

  /**
   * Returns whether an address is special, where special is defined as 0x0 to 0xf
   * inclusive. In other words, the last byte of the address must be < 0b10000 (16)
   * and every other byte must be zero.
   *
   * For more information on how special addresses are defined see AIP-40:
   * https://github.com/aptos-foundation/AIPs/blob/main/aips/aip-40.md.
   *
   * @returns true if the address is special, false if not.
   */
  isSpecial(): boolean {
    return (
      this.data.slice(0, this.data.length - 1).every((byte) => byte === 0) && this.data[this.data.length - 1] < 0b10000
    );
  }

  // ===
  // Methods for representing an instance of AccountAddress as other types.
  // ===

  /**
   * Return the AccountAddress as a string as per AIP-40.
   * https://github.com/aptos-foundation/AIPs/blob/main/aips/aip-40.md.
   *
   * In short, it means that special addresses are represented in SHORT form, meaning
   * 0x0 through to 0xf inclusive, and every other address is represented in LONG form,
   * meaning 0x + 64 hex characters.
   *
   * @returns AccountAddress as a string conforming to AIP-40.
   */
  toString(): string {
    return `0x${this.toStringWithoutPrefix()}`;
  }

  /**
   * NOTE: Prefer to use `toString` where possible.
   *
   * Return the AccountAddress as a string as per AIP-40 but without the leading 0x.
   *
   * Learn more by reading the docstring of `toString`.
   *
   * @returns AccountAddress as a string conforming to AIP-40 but without the leading 0x.
   */
  toStringWithoutPrefix(): string {
    let hex = bytesToHex(this.data);
    if (this.isSpecial()) {
      hex = hex[hex.length - 1];
    }
    return hex;
  }

  /**
   * NOTE: Prefer to use `toString` where possible.
   *
   * Whereas toString will format special addresses (as defined by isSpecial) using the
   * SHORT form (no leading 0s), this format the address in the LONG format
   * unconditionally.
   *
   * This means it will be 0x + 64 hex characters.
   *
   * @returns AccountAddress as a string in LONG form.
   */
  toStringLong(): string {
    return `0x${this.toStringLongWithoutPrefix()}`;
  }

  /*
   * NOTE: Prefer to use `toString` where possible.
   *
   * Whereas toString will format special addresses (as defined by isSpecial) using the
   * SHORT form (no leading 0s), this function will include leading zeroes. The string
   * will not have a leading zero.
   *
   * This means it will be 64 hex characters without a leading 0x.
   *
   * @returns AccountAddress as a string in LONG form without a leading 0x.
   */
  toStringLongWithoutPrefix(): string {
    return bytesToHex(this.data);
  }

  /**
   * Get the inner hex data. The inner data is already a Uint8Array so no conversion
   * is taking place here, it just returns the inner data.
   *
   * @returns Hex data as Uint8Array
   */
  toUint8Array(): Uint8Array {
    return this.data;
  }

  // ===
  // Methods for creating an instance of AccountAddress from other types.
  // ===

  /**
   * NOTE: This function has strict parsing behavior. For relaxed behavior, please use
   * the `fromStringRelaxed` function.
   *
   * Creates an instance of AccountAddress from a hex string.
   *
   * This function allows only the strictest formats defined by AIP-40. In short this
   * means only the following formats are accepted:
   *
   * - LONG
   * - SHORT for special addresses
   *
   * Where:
   * - LONG is defined as 0x + 64 hex characters.
   * - SHORT for special addresses is 0x0 to 0xf inclusive without padding zeroes.
   *
   * This means the following are not accepted:
   * - SHORT for non-special addresses.
   * - Any address without a leading 0x.
   *
   * Learn more about the different address formats by reading AIP-40:
   * https://github.com/aptos-foundation/AIPs/blob/main/aips/aip-40.md.
   *
   * @param args.input A hex string representing an account address.
   *
   * @returns An instance of AccountAddress.
   */
  static fromString(args: { input: string }): AccountAddress {
    // Assert the string starts with 0x.
    if (!args.input.startsWith("0x")) {
      throw new ParsingError("Hex string must start with a leading 0x.", AddressInvalidReason.LEADING_ZERO_X_REQUIRED);
    }

    const address = AccountAddress.fromStringRelaxed(args);

    // Check if the address is in LONG form. If it is not, this is only allowed for
    // special addresses, in which case we check it is in proper SHORT form.
    if (args.input.length != AccountAddress.LONG_STRING_LENGTH + 2) {
      if (!address.isSpecial()) {
        throw new ParsingError(
          "The given hex string is not a special address, it must be represented as 0x + 64 chars.",
          AddressInvalidReason.LONG_FORM_REQUIRED_UNLESS_SPECIAL,
        );
      } else {
        // 0x + one hex char is the only valid SHORT form for special addresses.
        if (args.input.length != 3) {
          throw new ParsingError(
            "The given hex string is a special address not in LONG form, it must be 0x0 to 0xf without padding zeroes.",
            AddressInvalidReason.INVALID_PADDING_ZEROES,
          );
        }
      }
    }

    return address;
  }

  /**
   * NOTE: This function has relaxed parsing behavior. For strict behavior, please use
   * the `fromString` function. Where possible use `fromString` rather than this
   * function, `fromStringRelaxed` is only provided for backwards compatibility.
   *
   * Creates an instance of AccountAddress from a hex string.
   *
   * This function allows all formats defined by AIP-40. In short this means the
   * following formats are accepted:
   *
   * - LONG, with or without leading 0x
   * - SHORT, with or without leading 0x
   *
   * Where:
   * - LONG is 64 hex characters.
   * - SHORT is 1 to 63 hex characters inclusive.
   * - Padding zeroes are allowed, e.g. 0x0123 is valid.
   *
   * Learn more about the different address formats by reading AIP-40:
   * https://github.com/aptos-foundation/AIPs/blob/main/aips/aip-40.md.
   *
   * @param args.input A hex string representing an account address.
   *
   * @returns An instance of AccountAddress.
   */
  static fromStringRelaxed(args: { input: string }): AccountAddress {
    let { input } = args;

    // Remove leading 0x for parsing.
    if (input.startsWith("0x")) {
      input = input.slice(2);
    }

    // Ensure the address string is at least 1 character long.
    if (input.length === 0) {
      throw new ParsingError(
        "Hex string is too short, must be 1 to 64 chars long, excluding the leading 0x.",
        AddressInvalidReason.TOO_SHORT,
      );
    }

    // Ensure the address string is not longer than 64 characters.
    if (input.length > 64) {
      throw new ParsingError(
        "Hex string is too long, must be 1 to 64 chars long, excluding the leading 0x.",
        AddressInvalidReason.TOO_LONG,
      );
    }

    let addressBytes: Uint8Array;
    try {
      // Pad the address with leading zeroes so it is 64 chars long and then convert
      // the hex string to bytes. Every two characters in a hex string constitutes a
      // single byte. So a 64 length hex string becomes a 32 byte array.
      addressBytes = hexToBytes(input.padStart(64, "0"));
    } catch (e) {
      const error = e as Error;
      // At this point the only way this can fail is if the hex string contains
      // invalid characters.
      throw new ParsingError(`Hex characters are invalid: ${error.message}`, AddressInvalidReason.INVALID_HEX_CHARS);
    }

    return new AccountAddress({ data: addressBytes });
  }

  /**
   * Convenience method for creating an AccountAddress from HexInput. For more
   * more information on how this works, see the constructor and fromString.
   *
   * @param args.input A hex string or Uint8Array representing an account address.
   *
   * @returns An instance of AccountAddress.
   */
  static fromHexInput(args: { input: HexInput }): AccountAddress {
    if (args.input instanceof Uint8Array) {
      return new AccountAddress({ data: args.input });
    }
    return AccountAddress.fromString({ input: args.input });
  }

  /**
   * Convenience method for creating an AccountAddress from HexInput. For more
   * more information on how this works, see the constructor and fromStringRelaxed.
   *
   * @param args.input A hex string or Uint8Array representing an account address.
   *
   * @returns An instance of AccountAddress.
   */
  static fromHexInputRelaxed(args: { input: HexInput }): AccountAddress {
    if (args.input instanceof Uint8Array) {
      return new AccountAddress({ data: args.input });
    }
    return AccountAddress.fromStringRelaxed({ input: args.input });
  }

  // ===
  // Methods for checking validity.
  // ===

  /**
   * Check if the string is a valid AccountAddress.
   *
   * @param str A hex string representing an account address.
   * @param relaxed If true, use relaxed parsing behavior. If false, use strict parsing behavior.
   *
   * @returns valid = true if the string is valid, valid = false if not. If the string
   * is not valid, invalidReason will be set explaining why it is invalid.
   */
  static isValid(args: { input: string; relaxed?: boolean }): ParsingResult<AddressInvalidReason> {
    try {
      if (args.relaxed) {
        AccountAddress.fromStringRelaxed({ input: args.input });
      } else {
        AccountAddress.fromString({ input: args.input });
      }
      return { valid: true };
    } catch (e) {
      const error = e as ParsingError<AddressInvalidReason>;
      return {
        valid: false,
        invalidReason: error.invalidReason,
        invalidReasonMessage: error.message,
      };
    }
  }

  /**
   * Return whether AccountAddresses are equal. AccountAddresses are considered equal
   * if their underlying byte data is identical.
   *
   * @param other The AccountAddress to compare to.
   * @returns true if the AccountAddresses are equal, false if not.
   */
  equals(other: AccountAddress): boolean {
    if (this.data.length !== other.data.length) return false;
    return this.data.every((value, index) => value === other.data[index]);
  }
}
