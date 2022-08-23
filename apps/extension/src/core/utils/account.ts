/* eslint-disable no-console */
// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { AptosAccount, HexString } from 'aptos';
import {
  WALLET_ENCRYPTED_ACCOUNTS_KEY,
  WALLET_ACCOUNTS_KEY,
  WALLET_STATE_ACCOUNT_ADDRESS_KEY,
  WALLET_STATE_LOADED_KEY,
  WALLET_STATE_NETWORK_LOCAL_STORAGE_KEY,
  WALLET_STATE_CUSTOM_NETWORKS_STORAGE_KEY,
} from 'core/constants';
import {
  AccountsState,
  AptosAccountState,
  DecryptedState,
  Mnemonic,
  MnemonicState,
  PublicAccount,
} from 'core/types/stateTypes';
import * as bip39 from '@scure/bip39';
import { wordlist } from '@scure/bip39/wordlists/english';
import { randomBytes, secretbox } from 'tweetnacl';
import pbkdf2 from 'pbkdf2';
import Browser from 'core/utils/browser';
import bs58 from 'bs58';
import { HDKey } from '@scure/bip32';
import {
  defaultCustomNetworks,
  defaultNetworkName,
  defaultNetworks,
} from 'core/hooks/useNetworks';
import {
  Accounts, pbkdf2Digest, pbkdf2Iterations, pbkdf2SaltSize,
} from 'core/hooks/useEncryptedStorageState';

// https://github.com/satoshilabs/slips/blob/master/slip-0044.md
const bip44Coin = 637;

interface EncryptedAccounts {
  encrypted: string,
  nonce: string,
  salt: string
}

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

export function createNewAccount(): AptosAccount {
  const account = new AptosAccount();
  // todo: make request to create account on chain
  return account;
}

export function getCurrentPublicAccount(): PublicAccount | null {
  const item = window.localStorage.getItem(WALLET_STATE_ACCOUNT_ADDRESS_KEY);
  if (item) {
    return JSON.parse(item);
  }
  return null;
}

export async function getBackgroundCurrentPublicAccount(): Promise<PublicAccount | null> {
  const result = await Browser.persistentStorage()?.get([WALLET_STATE_ACCOUNT_ADDRESS_KEY]);
  if (result && result[WALLET_STATE_ACCOUNT_ADDRESS_KEY]) {
    return JSON.parse(result[WALLET_STATE_ACCOUNT_ADDRESS_KEY]);
  }
  return null;
}

export function getEncryptedAccounts(): EncryptedAccounts | null {
  const item = window.localStorage.getItem(WALLET_ENCRYPTED_ACCOUNTS_KEY);
  if (item) {
    const accounts: EncryptedAccounts = JSON.parse(item);
    return accounts;
  }
  return null;
}

export async function getBackgroundEncryptedAccounts(): Promise<EncryptedAccounts | null> {
  const result = await Browser.persistentStorage()?.get([WALLET_ENCRYPTED_ACCOUNTS_KEY]);
  if (result && result[WALLET_ENCRYPTED_ACCOUNTS_KEY]) {
    const accounts: EncryptedAccounts = JSON.parse(result[WALLET_ENCRYPTED_ACCOUNTS_KEY]);
    return accounts;
  }
  return null;
}

export function getDecryptedAccounts(): AccountsState | null {
  const item = window.sessionStorage.getItem(WALLET_ACCOUNTS_KEY);
  if (item) {
    const decryptedState: DecryptedState = JSON.parse(item);
    return decryptedState?.accounts ?? null;
  }
  return null;
}

export async function getBackgroundDecryptedState(): Promise<DecryptedState | null> {
  const result = await Browser.sessionStorage()?.get([WALLET_ACCOUNTS_KEY]);
  if (result && result[WALLET_ACCOUNTS_KEY]) {
    return JSON.parse(result[WALLET_ACCOUNTS_KEY]);
  }
  return null;
}

export async function getBackgroundDecryptedAccounts(): Promise<AccountsState | null> {
  return (await getBackgroundDecryptedState())?.accounts ?? null;
}

export function getDecryptionKeyFromSession(): Uint8Array | null {
  const item = window.sessionStorage.getItem(WALLET_ACCOUNTS_KEY);
  if (item) {
    const decryptedState: DecryptedState = JSON.parse(item);
    return decryptedState ? bs58.decode(decryptedState.decryptionKey) : null;
  }
  return null;
}

export function isWalletLocked(): boolean {
  const localStorageState = getEncryptedAccounts();
  const publicAccount = getCurrentPublicAccount();
  return (localStorageState?.encrypted !== null
          && publicAccount !== null
          && getDecryptedAccounts() === null);
}

export async function isBackgroundWalletLocked(): Promise<boolean> {
  const localStorageState = await getBackgroundEncryptedAccounts();
  const publicAccount = await getBackgroundCurrentPublicAccount();
  return (localStorageState?.encrypted !== null
          && publicAccount !== null
          && await getBackgroundDecryptedAccounts() === null);
}

async function deriveEncryptionKey(
  password: string,
  salt: Uint8Array,
): Promise<Uint8Array> {
  return new Promise((resolve, reject) => {
    pbkdf2.pbkdf2(
      password,
      salt,
      pbkdf2Iterations,
      secretbox.keyLength,
      pbkdf2Digest,
      (error: Error, key: Uint8Array) => {
        if (error) {
          reject(error);
        } else {
          resolve(key);
        }
      },
    );
  });
}

export async function storeEncryptedAccounts(
  accounts: AccountsState,
  password: string | undefined,
) {
  const plaintext = JSON.stringify(accounts);
  let decryptionKey; let nonce; let salt;
  // if password is provided we need to make a new encryption key
  // else we should already have it in the session
  if (password) {
    salt = randomBytes(pbkdf2SaltSize);
    nonce = randomBytes(secretbox.nonceLength);
    decryptionKey = await deriveEncryptionKey(password, salt);
  } else {
    // todo: add error handling here for nulls
    decryptionKey = getDecryptionKeyFromSession()!;
    const encryptedAccounts = getEncryptedAccounts()!;
    nonce = bs58.decode(encryptedAccounts.nonce);
    salt = bs58.decode(encryptedAccounts.salt);
  }

  const encrypted = secretbox(Buffer.from(plaintext), nonce, decryptionKey);
  const encryptedAccounts: EncryptedAccounts = {
    encrypted: bs58.encode(encrypted),
    nonce: bs58.encode(nonce),
    salt: bs58.encode(salt),
  };
  const decryptedState: DecryptedState = { accounts, decryptionKey: bs58.encode(decryptionKey) };
  const decryptedString = JSON.stringify(decryptedState);
  localStorage.setItem(WALLET_ENCRYPTED_ACCOUNTS_KEY, JSON.stringify(encryptedAccounts));
  window.sessionStorage.setItem(WALLET_ACCOUNTS_KEY, decryptedString);
  Browser.sessionStorage()?.set({ [WALLET_ACCOUNTS_KEY]: decryptedString });
}

export async function unlockAccounts(
  password: string,
  background: boolean = false,
): Promise<AccountsState | null> {
  let encryptedAccounts;
  if (background) {
    encryptedAccounts = await getBackgroundEncryptedAccounts();
  } else {
    encryptedAccounts = getEncryptedAccounts();
  }
  if (encryptedAccounts) {
    try {
      const encrypted = bs58.decode(encryptedAccounts.encrypted);
      const nonce = bs58.decode(encryptedAccounts.nonce);
      const salt = bs58.decode(encryptedAccounts.salt);
      const key = await deriveEncryptionKey(password, salt);
      const result = secretbox.open(encrypted, nonce, key);
      if (!result) {
        throw Error('Something went wrong');
      }
      const decodedPlaintext = Buffer.from(result).toString();
      const accounts: AccountsState = JSON.parse(decodedPlaintext);
      const decryptedState: DecryptedState = { accounts, decryptionKey: bs58.encode(key) };
      const decryptedString = JSON.stringify(decryptedState);
      window.sessionStorage.setItem(WALLET_ACCOUNTS_KEY, decryptedString);
      Browser.sessionStorage()?.set({ [WALLET_ACCOUNTS_KEY]: decryptedString });
      return accounts;
    } catch (error) {
      // eslint-disable-next-line no-console
      console.warn(error);
      return null;
    }
  }
  return null;
}

export function getAptosAccountState(accounts: Accounts, address: string): AptosAccountState {
  const account = address && accounts ? accounts[address] : undefined;
  return account ? new AptosAccount(
    HexString.ensure(account.privateKey).toUint8Array(),
    account.address,
  ) : undefined;
}

export function getMnemonicState(address: string): MnemonicState {
  const accounts = getDecryptedAccounts();
  if (!address || !accounts) {
    return undefined;
  }
  const { mnemonic } = accounts[address];
  return mnemonic;
}

export function getBackgroundAptosAccountState(): Promise<AptosAccountState> {
  return new Promise((resolve) => {
    getBackgroundCurrentPublicAccount().then((publicAccount) => {
      Browser.sessionStorage()?.get([WALLET_ACCOUNTS_KEY], (accountResult: any) => {
        if (!accountResult[WALLET_ACCOUNTS_KEY]) {
          resolve(undefined);
        }
        const accounts = JSON.parse(accountResult[WALLET_ACCOUNTS_KEY]);
        if (accounts && publicAccount?.address) {
          const aptosAccount = getAptosAccountState(accounts, publicAccount.address);
          resolve(aptosAccount);
        } else {
          resolve(undefined);
        }
      });
    });
  });
}

export async function getBackgroundNetwork() {
  const result = await Browser.persistentStorage()?.get([
    WALLET_STATE_NETWORK_LOCAL_STORAGE_KEY,
    WALLET_STATE_CUSTOM_NETWORKS_STORAGE_KEY,
  ]);

  const serializedNetworkId = result && result[WALLET_STATE_NETWORK_LOCAL_STORAGE_KEY];
  const networkName = serializedNetworkId
    ? JSON.parse(serializedNetworkId)
    : undefined;

  const serializedCustomNetworks = result && result[WALLET_STATE_CUSTOM_NETWORKS_STORAGE_KEY];
  const customNetworks = serializedCustomNetworks
    ? JSON.parse(serializedCustomNetworks)
    : defaultCustomNetworks;

  const networks = { ...defaultNetworks, ...customNetworks };
  return networks[networkName ?? defaultNetworkName];
}

export async function loadBackgroundState(): Promise<boolean> {
  if (!getDecryptedAccounts()) {
    const state = await getBackgroundDecryptedState();
    window.sessionStorage.setItem(WALLET_ACCOUNTS_KEY, JSON.stringify(state));
  }
  sessionStorage.setItem(WALLET_STATE_LOADED_KEY, String(true));
  return true;
}
