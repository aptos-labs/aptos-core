import { Serializer, Deserializer } from "../../bcs";

/**
 * Representation of an Identifier that can serialized and deserialized.
 * We use Identifier to represent the module "name" in "ModuleId" and
 * the "function name" in "EntryFunction"
 */
export class Identifier {
  public identifier: string;

  constructor(identifier: string) {
    this.identifier = identifier;
  }

  public serialize(serializer: Serializer): void {
    serializer.serializeStr(this.identifier);
  }

  static deserialize(deserializer: Deserializer): Identifier {
    const identifier = deserializer.deserializeStr();
    return new Identifier(identifier);
  }
}
