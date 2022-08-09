// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { AptosAccount } from 'aptos';
import { WALLET_STATE_LOCAL_STORAGE_KEY, WALLET_STATE_NETWORK_LOCAL_STORAGE_KEY } from 'core/constants';
import {
  AptosAccountState, LocalStorageState, Mnemonic, MnemonicState,
} from 'core/types/stateTypes';
import * as bip39 from '@scure/bip39';
import { wordlist } from '@scure/bip39/wordlists/english';

import Browser from 'core/utils/browser';
import {
  defaultNetworkType, NodeUrl, nodeUrlMap, nodeUrlReverseMap,
} from './network';

export function generateMnemonic() {
  const mnemonic = bip39.generateMnemonic(wordlist);
  return mnemonic;
}

export async function generateMnemonicObject(mnemonicString: string): Promise<Mnemonic> {
  const seed = await bip39.mnemonicToSeed(mnemonicString);
  const bufferSeed = new Uint8Array(seed.buffer);
  return { mnemonic: mnemonicString, seed: bufferSeed };
}

export async function createNewMnemonic(): Promise<Mnemonic> {
  const mnemonic = bip39.generateMnemonic(wordlist);
  const seed = await bip39.mnemonicToSeed(mnemonic);
  const bufferSeed = new Uint8Array(seed.buffer);
  return { mnemonic, seed: bufferSeed };
}

export function createNewAccount(): AptosAccount {
  const account = new AptosAccount();
  // todo: make request to create account on chain
  return account;
}

export function getLocalStorageState(): LocalStorageState | null {
  // Get from local storage by key
  const item = window.localStorage.getItem(WALLET_STATE_LOCAL_STORAGE_KEY);
  if (item) {
    const localStorageState: LocalStorageState = JSON.parse(item);
    return localStorageState;
  }
  return null;
}

export function getAptosAccountState(localStorage: LocalStorageState): AptosAccountState {
  const { accounts, currAccountAddress } = localStorage;
  const currAccountAddressString = currAccountAddress?.toString();
  if (!currAccountAddressString || !accounts) {
    return undefined;
  }
  const aptosAccountObject = accounts[currAccountAddressString].aptosAccount;
  return aptosAccountObject ? AptosAccount.fromAptosAccountObject(aptosAccountObject) : undefined;
}

export function getMnemonicState(localStorage: LocalStorageState): MnemonicState {
  const { accounts, currAccountAddress } = localStorage;
  const currAccountAddressString = currAccountAddress?.toString();
  if (!currAccountAddressString || !accounts) {
    return undefined;
  }
  const { mnemonic } = accounts[currAccountAddressString];
  return mnemonic;
}

export function getBackgroundAptosAccountState(): Promise<AptosAccountState> {
  return new Promise((resolve) => {
    Browser.storage()?.get([WALLET_STATE_LOCAL_STORAGE_KEY], (result: any) => {
      if (!result[WALLET_STATE_LOCAL_STORAGE_KEY]) {
        resolve(undefined);
      }
      const localStorage: LocalStorageState = JSON.parse(result[WALLET_STATE_LOCAL_STORAGE_KEY]);
      if (localStorage) {
        const aptosAccount = getAptosAccountState(localStorage);
        resolve(aptosAccount);
      } else {
        resolve(undefined);
      }
    });
  });
}

export function getBackgroundNodeUrl(): Promise<string> {
  return new Promise((resolve) => {
    Browser.storage()?.get([WALLET_STATE_NETWORK_LOCAL_STORAGE_KEY], (result: any) => {
      const network = result[WALLET_STATE_NETWORK_LOCAL_STORAGE_KEY];
      if (network) {
        resolve(network);
      } else {
        resolve(nodeUrlMap.Devnet);
      }
    });
  });
}

export async function getBackgroundNetworkName(): Promise<string> {
  const network = (await getBackgroundNodeUrl()) as NodeUrl;
  if (network) {
    return nodeUrlReverseMap[network];
  }
  return defaultNetworkType;
}
