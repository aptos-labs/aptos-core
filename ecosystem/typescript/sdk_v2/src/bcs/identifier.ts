// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

import { Deserializer } from "./deserializer";
import { Serializer } from "./serializer";

export class Identifier {
  constructor(public value: string) {}

  public serialize(serializer: Serializer): void {
    serializer.serializeStr(this.value);
  }

  static deserialize(deserializer: Deserializer): Identifier {
    const value = deserializer.deserializeStr();
    return new Identifier(value);
  }
}
