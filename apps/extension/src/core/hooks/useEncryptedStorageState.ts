// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { usePersistentStorageState, useSessionStorageState } from 'core/hooks/useStorageState';
import bs58 from 'bs58';
import { randomBytes, secretbox } from 'tweetnacl';
import pbkdf2 from 'pbkdf2';
import { Account } from 'core/types/stateTypes';
import { WALLET_ACCOUNTS_KEY } from 'core/constants';

export const pbkdf2Iterations = 10000;
export const pbkdf2Digest = 'sha256';
export const pbkdf2SaltSize = 16;

export type Accounts = Record<string, Account>;

export interface EncryptedState {
  ciphertext: string;
  nonce: string;
  salt: string;
}

async function deriveEncryptionKey(password: string, salt: Uint8Array) {
  return new Promise<Uint8Array>((resolve, reject) => {
    pbkdf2.pbkdf2(
      password,
      salt,
      pbkdf2Iterations,
      secretbox.keyLength,
      pbkdf2Digest,
      (error, key) => {
        if (error) {
          reject(error);
        } else {
          resolve(key);
        }
      },
    );
  });
}

/**
 * Create a state synced with an encrypted representation in persistent storage.
 */
export default function useEncryptedStorageState() {
  const [
    encryptedState,
    setEncryptedState,
    isEncryptedStateReady,
  ] = usePersistentStorageState<EncryptedState>(`${WALLET_ACCOUNTS_KEY}.encryptedState`);
  const [
    encryptionKey,
    setEncryptionKey,
    isEncryptionKeyReady,
  ] = useSessionStorageState<string | undefined>(`${WALLET_ACCOUNTS_KEY}.encryptionKey`);
  const [
    value,
    setValue,
    isValueReady,
  ] = useSessionStorageState<Accounts>(`${WALLET_ACCOUNTS_KEY}`);

  const isReady = isEncryptedStateReady && isEncryptionKeyReady && isValueReady;
  const isInitialized = encryptedState !== undefined;
  const isUnlocked = isInitialized && encryptionKey !== undefined && value !== undefined;

  // Try to automatically unlock as soon as the storage states are ready
  if (isReady && isInitialized && !isUnlocked && encryptionKey) {
    const ciphertext = bs58.decode(encryptedState.ciphertext);
    const nonce = bs58.decode(encryptedState.nonce);
    const plaintext = secretbox.open(ciphertext, nonce, bs58.decode(encryptionKey))!;
    const decodedPlaintext = Buffer.from(plaintext).toString();
    const newValue = JSON.parse(decodedPlaintext) as Accounts;
    setValue(newValue);
  }

  async function initialize(password: string, initialValue: Accounts) {
    // generate salt and nonce
    const salt = randomBytes(pbkdf2SaltSize);
    const nonce = randomBytes(secretbox.nonceLength);

    // Generate encryption key from password + salt
    const newEncryptionKey = await deriveEncryptionKey(password, salt);

    // Initialize encrypted data
    const newValue = initialValue;
    await setValue(newValue);

    const plaintext = JSON.stringify(newValue);
    const ciphertext = secretbox(Buffer.from(plaintext), nonce, newEncryptionKey);

    // persist encrypted state
    const newEncryptedState = {
      ciphertext: bs58.encode(ciphertext),
      nonce: bs58.encode(nonce),
      salt: bs58.encode(salt),
    };

    await Promise.all([
      setEncryptedState(newEncryptedState),
      setEncryptionKey(bs58.encode(newEncryptionKey)),
    ]);
  }

  const unlock = async (password: string) => {
    const ciphertext = bs58.decode(encryptedState!.ciphertext);
    const salt = bs58.decode(encryptedState!.salt);
    const nonce = bs58.decode(encryptedState!.nonce);

    // use password + salt to retrieve encryption key
    const newEncryptionKey = await deriveEncryptionKey(password, salt);

    const plaintext = secretbox.open(ciphertext, nonce, newEncryptionKey)!;
    const decodedPlaintext = Buffer.from(plaintext).toString();

    // check that data is unencrypted correctly
    const newValue = JSON.parse(decodedPlaintext) as Accounts;
    await setValue(newValue);

    // save decryption key to session
    await setEncryptionKey(bs58.encode(newEncryptionKey));
  };

  const update = async (newValue: Accounts) => {
    const plaintext = JSON.stringify(newValue);
    const nonce = randomBytes(secretbox.nonceLength);
    const ciphertext = secretbox(Buffer.from(plaintext), nonce, bs58.decode(encryptionKey!));

    const newEncryptedState = {
      ...encryptedState!,
      ciphertext: bs58.encode(ciphertext),
      nonce: bs58.encode(nonce),
    };

    await setEncryptedState(newEncryptedState);
    await setValue(newValue);
  };

  const lock = async () => {
    await setEncryptionKey(undefined);
    await setValue(undefined);
  };

  return {
    initialize,
    isInitialized,
    isReady,
    isUnlocked,
    lock,
    unlock,
    update,
    value,
  };
}
