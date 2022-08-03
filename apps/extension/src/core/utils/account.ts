// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { AptosAccount } from 'aptos';
import {
  WALLET_ENCRYPTED_ACCOUNTS_KEY,
  WALLET_SESSION_ACCOUNTS_KEY,
  WALLET_STATE_ACCOUNT_ADDRESS_KEY,
  WALLET_STATE_NETWORK_LOCAL_STORAGE_KEY,
} from 'core/constants';
import {
  AccountsState,
  AptosAccountState,
  DecryptedState,
  Mnemonic,
  MnemonicState,
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

export function getCurrAccountAddress(): string | null {
  const address = window.localStorage.getItem(WALLET_STATE_ACCOUNT_ADDRESS_KEY);
  return address ?? null;
}

export function getEncryptedAccounts(): AccountsState | null {
  const item = window.localStorage.getItem(WALLET_ENCRYPTED_ACCOUNTS_KEY);
  if (item) {
    const accounts: AccountsState = JSON.parse(item);
    return accounts;
  }
  return null;
}

export function getDecryptedAccounts(): AccountsState | null {
  const item = window.sessionStorage.getItem(WALLET_SESSION_ACCOUNTS_KEY);
  if (item) {
    const decryptedState: DecryptedState = JSON.parse(item);
    return decryptedState?.accounts ?? null;
  }
  return null;
}

export function isWalletLocked(): boolean {
  const localStorageState = getEncryptedAccounts();
  const currAccountAddress = getCurrAccountAddress();
  return (localStorageState?.encrypted_accounts !== null
          && currAccountAddress !== null
          && getDecryptedAccounts() === null);
}

export async function storeEncryptedAccounts(
  accounts: AccountsState,
  password: string | undefined,
) {
  // todo: encrypt/decrypt encrypted_accounts
  // eslint-disable-next-line no-console
  console.log(password);

  const decryptedState: DecryptedState = {
    accounts,
    decryptionKey: password ?? '',
  };
  const decryptedStateString = JSON.stringify(decryptedState);
  const accountsString = JSON.stringify(accounts);
  localStorage.setItem(
    WALLET_ENCRYPTED_ACCOUNTS_KEY,
    accountsString,
  );
  window.sessionStorage.setItem(
    WALLET_SESSION_ACCOUNTS_KEY,
    decryptedStateString,
  );
  Browser.sessionStorage()?.set({ [WALLET_SESSION_ACCOUNTS_KEY]: decryptedStateString });
}

export function unlockAccounts(password: string): AccountsState | null {
  const encryptedAccounts = getEncryptedAccounts();

  // todo: encrypt/decrypt encrypted_accounts
  // eslint-disable-next-line no-console
  console.log(password);

  const decryptedState: DecryptedState = encryptedAccounts ? {
    accounts: encryptedAccounts,
    decryptionKey: password,
  } : null;
  const decryptedString = JSON.stringify(decryptedState);
  window.sessionStorage.setItem(
    WALLET_SESSION_ACCOUNTS_KEY,
    decryptedString,
  );
  Browser.sessionStorage()?.set([WALLET_SESSION_ACCOUNTS_KEY], decryptedString);
  return encryptedAccounts ?? null;
}

export function getAptosAccountState(accounts: AccountsState, address: string): AptosAccountState {
  if (address && accounts) {
    const aptosAccountObject = accounts[address].aptosAccount;
    return aptosAccountObject ? AptosAccount.fromAptosAccountObject(aptosAccountObject) : undefined;
  }
  return undefined;
}

export function getMnemonicState(address: string): MnemonicState {
  const accounts = getDecryptedAccounts();
  if (!address || !accounts) {
    return undefined;
  }
  const { mnemonic } = accounts[address];
  return mnemonic;
}

// todo: fix this to prompt the password for the encryption if needed
export function getBackgroundAptosAccountState(): Promise<AptosAccountState> {
  return new Promise((resolve) => {
    Browser.persistentStorage()?.get([WALLET_STATE_ACCOUNT_ADDRESS_KEY], (addressResult: any) => {
      const address: string = addressResult[WALLET_STATE_ACCOUNT_ADDRESS_KEY];
      Browser.sessionStorage()?.get([WALLET_SESSION_ACCOUNTS_KEY], (accountResult: any) => {
        if (!accountResult[WALLET_SESSION_ACCOUNTS_KEY]) {
          resolve(undefined);
        }
        const result = accountResult[WALLET_SESSION_ACCOUNTS_KEY];
        const decryptedState: DecryptedState = JSON.parse(result);
        const accounts = decryptedState?.accounts;
        if (accounts && address) {
          const aptosAccount = getAptosAccountState(accounts, address);
          resolve(aptosAccount);
        } else {
          resolve(undefined);
        }
      });
    });
  });
}

export function getBackgroundNodeUrl(): Promise<string> {
  return new Promise((resolve) => {
    Browser.persistentStorage()?.get([WALLET_STATE_NETWORK_LOCAL_STORAGE_KEY], (result: any) => {
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
