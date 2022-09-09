// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { Serializer } from "../bcs";
import { AccountAddress } from "./account_address";

export class RotationProofChallenge {
  constructor(
    public readonly accountAddress: AccountAddress,
    public readonly moduleName: string,
    public readonly structName: string,
    public readonly sequenceNumber: number | bigint,
    public readonly originator: AccountAddress,
    public readonly currentAuthKey: AccountAddress,
    public readonly newPublicKey: Uint8Array,
  ) {}

  serialize(serializer: Serializer): void {
    this.accountAddress.serialize(serializer);
    serializer.serializeStr(this.moduleName);
    serializer.serializeStr(this.structName);
    serializer.serializeU64(this.sequenceNumber);
    this.originator.serialize(serializer);
    this.currentAuthKey.serialize(serializer);
    serializer.serializeBytes(this.newPublicKey);
  }
}
