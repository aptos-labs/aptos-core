import { Serializer, Deserializer } from "../../bcs";
import { AccountAddress } from "../../core";
import { Identifier } from "./identifier";

/**
 * Representation of a ModuleId that can serialized and deserialized
 * ModuleId means the module address (e.g "0x1") and the module name (e.g "coin")
 */
export class ModuleId {
  public readonly address: AccountAddress;
  public readonly name: Identifier;

  /**
   * Full name of a module.
   * @param address The account address. e.g "0x1"
   * @param name The module name under the "address". e.g "coin"
   */
  constructor(address: AccountAddress, name: Identifier) {
    this.address = address;
    this.name = name;
  }

  /**
   * Converts a string literal to a ModuleId
   * @param moduleId String literal in format "account_address::module_name", e.g. "0x1::coin"
   * @returns ModuleId
   */
  static fromStr(moduleId: `${string}::${string}`): ModuleId {
    const parts = moduleId.split("::");
    if (parts.length !== 2) {
      throw new Error("Invalid module id.");
    }
    return new ModuleId(AccountAddress.fromString({ input: parts[0] }), new Identifier(parts[1]));
  }

  serialize(serializer: Serializer): void {
    this.address.serialize(serializer);
    this.name.serialize(serializer);
  }

  static deserialize(deserializer: Deserializer): ModuleId {
    const address = AccountAddress.deserialize(deserializer);
    const name = Identifier.deserialize(deserializer);
    return new ModuleId(address, name);
  }
}
