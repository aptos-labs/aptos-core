import { Serializer, Deserializer } from "../../bcs";

/**
 * Representation of a ChainId that can serialized and deserialized
 */
export class ChainId {
  public readonly chainId: number;

  constructor(chainId: number) {
    this.chainId = chainId;
  }

  serialize(serializer: Serializer): void {
    serializer.serializeU8(this.chainId);
  }

  static deserialize(deserializer: Deserializer): ChainId {
    const chainId = deserializer.deserializeU8();
    return new ChainId(chainId);
  }
}
