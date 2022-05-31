import { HexString, MaybeHexString } from "../../hex_string";
import { Serializer, Deserializer, bytes } from "../bcs";

export class AccountAddress {
  static readonly LENGTH: number = 32;

  readonly address: bytes;

  constructor(address: bytes) {
    if (address.length !== AccountAddress.LENGTH) {
      throw new Error("Expected address of length 32");
    }
    this.address = address;
  }

  /**
   * Creates AccountAddress from a hex string.
   * @param address Hex string can be with a prefix or without a prefix,
   *   e.g. '0x1aa' or '1aa'. Hex string will be left padded with 0s if too short.
   */
  static fromHex(address: MaybeHexString): AccountAddress {
    address = HexString.ensure(address);

    // If an address hex has odd number of digits, padd the hex string with 0
    // e.g. '1aa' would become '01aa'.
    if (address.noPrefix().length % 2 !== 0) {
      address = new HexString(`0${address.noPrefix()}`);
    }

    const addressBytes = address.toUint8Array();

    if (addressBytes.length > AccountAddress.LENGTH) {
      throw new Error(`Hex string is too long. Address's length is 32 bytes.`);
    } else if (addressBytes.length === AccountAddress.LENGTH) {
      return new AccountAddress(addressBytes);
    }

    const res: bytes = new Uint8Array(AccountAddress.LENGTH);
    res.set(addressBytes, AccountAddress.LENGTH - addressBytes.length);

    return new AccountAddress(res);
  }

  serialize(serializer: Serializer): void {
    serializer.serializeFixedBytes(this.address);
  }

  static deserialize(deserializer: Deserializer): AccountAddress {
    deserializer.deserializeFixedBytes(AccountAddress.LENGTH);
    return new AccountAddress(deserializer.deserializeFixedBytes(AccountAddress.LENGTH));
  }
}
