// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { AptosAccount, HexString } from 'aptos';
import {
  AptosAccountState,
  Mnemonic,
  PublicAccount,
} from 'core/types/stateTypes';
import * as bip39 from '@scure/bip39';
import { wordlist } from '@scure/bip39/wordlists/english';
import { HDKey } from '@scure/bip32';
import { PersistentStorage, SessionStorage } from 'shared/storage';
import {
  defaultCustomNetworks,
  defaultNetworkName,
  defaultNetworks,
} from 'shared/types';

// https://github.com/satoshilabs/slips/blob/master/slip-0044.md
const bip44Coin = 637;

export function generateMnemonic() {
  const mnemonic = bip39.generateMnemonic(wordlist);
  return mnemonic;
}

// We are only looking for the first derivation of the bip44
// In the future we may support importing multiple keys from other wallets
function getAptosBip44Path(): string {
  return `m/44'/${bip44Coin}'/0'/0/0`;
}

export async function generateMnemonicObject(mnemonicString: string): Promise<Mnemonic> {
  const seed = await bip39.mnemonicToSeed(mnemonicString);
  const derivationPath = getAptosBip44Path();
  const node = HDKey.fromMasterSeed(Buffer.from(seed));
  const key = node.derive(derivationPath).privateKey;
  if (key) {
    return { mnemonic: mnemonicString, seed: key };
  }
  throw new Error('Private key can not be derived');
}

export async function getBackgroundCurrentPublicAccount(): Promise<PublicAccount | null> {
  const {
    activeAccountAddress: address,
    activeAccountPublicKey: publicKey,
  } = await PersistentStorage.get([
    'activeAccountAddress',
    'activeAccountPublicKey',
  ]);

  return (address !== undefined && publicKey !== undefined)
    ? { address, publicKey }
    : null;
}

export async function getBackgroundAptosAccountState(): Promise<AptosAccountState> {
  const [{ activeAccountAddress }, { accounts }] = await Promise.all([
    PersistentStorage.get(['activeAccountAddress']),
    SessionStorage.get(['accounts']),
  ]);

  const activeAccount = activeAccountAddress !== undefined && accounts !== undefined
    ? accounts[activeAccountAddress]
    : undefined;

  return activeAccount !== undefined
    ? new AptosAccount(
      HexString.ensure(activeAccount.privateKey).toUint8Array(),
      activeAccount.address,
    ) : undefined;
}

export async function getBackgroundNetwork() {
  const { activeNetworkName, customNetworks } = await PersistentStorage.get([
    'activeNetworkName',
    'customNetworks',
  ]);
  const networks = {
    ...defaultNetworks,
    ...(customNetworks ?? defaultCustomNetworks),
  };
  return networks[activeNetworkName ?? defaultNetworkName];
}
