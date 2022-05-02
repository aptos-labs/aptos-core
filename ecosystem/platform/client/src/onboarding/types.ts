// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

export type AptosAddress = string;

export type Identity = {
  mainnetAddress: AptosAddress;
};

export function isValidIdentity(
  identity: Partial<Identity>,
): identity is Identity {
  return identity.mainnetAddress != null;
}
