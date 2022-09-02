// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

export interface EncryptedAccounts {
  ciphertext: string;
  nonce: string;
}

export interface PublicAccount {
  address: string,
  publicKey: string
}

export type Account = PublicAccount & {
  mnemonic?: string;
  name?: string;
  privateKey: string;
};

export type Accounts = Record<string, Account>;
