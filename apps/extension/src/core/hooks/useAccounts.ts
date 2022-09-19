// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import bs58 from 'bs58';
import constate from 'constate';
import { randomBytes, secretbox } from 'tweetnacl';
import pbkdf2 from 'pbkdf2';

import { triggerAccountChange } from 'core/utils/providerEvents';
import { AptosAccount, HexString } from 'aptos';
import { useAppState } from 'core/hooks/useAppState';
import {
  Account, Accounts, EncryptedAccounts,
} from 'shared/types';
import { latestVersion } from 'core/constants';

const pbkdf2Iterations = 10000;
const pbkdf2Digest = 'sha256';
const pbkdf2SaltSize = 16;

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
 * Hook for initializing the accounts.
 * The accounts state is stored in encrypted format, thus the only possible
 * operation that can be done by default is to initialize it
 */
export const [AccountsProvider, useAccounts] = constate(() => {
  const {
    accounts,
    activeAccountAddress,
    encryptedAccounts,
    encryptedStateVersion,
    encryptionKey,
    salt,
    updatePersistentState,
    updateSessionState,
  } = useAppState();

  /**
   * Initialize the accounts state with a password to encrypt it.
   * @param password the password for encrypting the accounts
   * @param initialAccounts initial accounts
   */
  async function initAccounts(password: string, initialAccounts: Accounts) {
    // Generate salt and use it to derive encryption key
    const newSalt = randomBytes(pbkdf2SaltSize);
    const newEncryptionKey = await deriveEncryptionKey(password, newSalt);

    // Initialize encrypted state
    const plaintext = JSON.stringify(initialAccounts);
    const nonce = randomBytes(secretbox.nonceLength);
    const ciphertext = secretbox(Buffer.from(plaintext), nonce, newEncryptionKey);

    const newEncryptedAccounts = {
      ciphertext: bs58.encode(ciphertext),
      nonce: bs58.encode(nonce),
    };

    // Update and persist state
    const firstAvailableAddress = Object.keys(initialAccounts)[0];
    const firstAvailableAccount = firstAvailableAddress
      ? initialAccounts[firstAvailableAddress]
      : undefined;
    await Promise.all([
      updatePersistentState({
        activeAccountAddress: firstAvailableAccount?.address,
        activeAccountPublicKey: firstAvailableAccount?.publicKey,
        encryptedAccounts: newEncryptedAccounts,
        encryptedStateVersion: latestVersion,
        salt: bs58.encode(newSalt),
      }),
      updateSessionState({
        accounts: initialAccounts,
        encryptionKey: bs58.encode(newEncryptionKey),
      }),
    ]);
  }

  return {
    accounts,
    activeAccountAddress,
    encryptedAccounts,
    encryptedStateVersion,
    encryptionKey,
    initAccounts,
    salt,
  };
});

export interface UseInitializedAccountsProps {
  encryptedAccounts: EncryptedAccounts,
  encryptedStateVersion: number,
  salt: string
}

/**
 * Hook for locking/unlocking the accounts state.
 * Requires the accounts state to be initialized with a password.
 */
export const [InitializedAccountsProvider, useInitializedAccounts] = constate(({
  encryptedAccounts,
  encryptedStateVersion,
  salt,
}: UseInitializedAccountsProps) => {
  const {
    updatePersistentState,
    updateSessionState,
  } = useAppState();

  const clearAccounts = async () => {
    await updatePersistentState({
      activeAccountAddress: undefined,
      activeAccountPublicKey: undefined,
      encryptedAccounts: undefined,
      salt: undefined,
    });
    // Note: session needs to be updated after persistent state
    await updateSessionState({
      accounts: undefined,
      encryptionKey: undefined,
    });
  };

  const migrateEncryptedState = async (accounts: Accounts, encryptionKey: Uint8Array) => {
    if (encryptedStateVersion === latestVersion) {
      return accounts;
    }

    const newAccounts = accounts;
    // migration to version 1
    if (encryptedStateVersion < 1) {
      Object.keys(newAccounts).forEach((key) => {
        delete newAccounts[key].mnemonic;
      });
    }

    // Re-encrypt migrated accounts
    const newPlaintext = JSON.stringify(newAccounts);
    const newNonce = randomBytes(secretbox.nonceLength);
    const newCiphertext = secretbox(Buffer.from(newPlaintext), newNonce, encryptionKey);
    const newEncryptedAccounts = {
      ciphertext: bs58.encode(newCiphertext),
      nonce: bs58.encode(newNonce),
    };
    await updatePersistentState({
      encryptedAccounts: newEncryptedAccounts,
      encryptedStateVersion: 1,
    });
    return newAccounts;
  };

  const unlockAccounts = async (password: string) => {
    console.log(password);
    const ciphertext = bs58.decode(encryptedAccounts.ciphertext);
    const nonce = bs58.decode(encryptedAccounts.nonce);

    // Use password + salt to retrieve encryption key
    const newEncryptionKey = await deriveEncryptionKey(password, bs58.decode(salt));

    // Retrieved unencrypted value
    const plaintext = secretbox.open(ciphertext, nonce, newEncryptionKey)!;
    const decodedPlaintext = Buffer.from(plaintext).toString();
    let newAccounts = JSON.parse(decodedPlaintext) as Accounts;

    // Migrate if needed
    newAccounts = await migrateEncryptedState(newAccounts, newEncryptionKey);

    // Update state
    await updateSessionState({
      accounts: newAccounts,
      encryptionKey: bs58.encode(newEncryptionKey),
    });
  };

  const lockAccounts = async () => {
    await updateSessionState({
      accounts: undefined,
      encryptionKey: undefined,
    });
  };

  const changePassword = async (oldPassword: string, newPassword: string) => {
    const ciphertext = bs58.decode(encryptedAccounts.ciphertext);
    const nonce = bs58.decode(encryptedAccounts.nonce);
    const encryptionKey = await deriveEncryptionKey(oldPassword, bs58.decode(salt));

    // Retrieved unencrypted value
    const plaintext = secretbox.open(ciphertext, nonce, encryptionKey)!;

    // incorrect current password
    if (!plaintext) {
      throw new Error('Incorrect current password');
    }

    const decodedPlaintext = Buffer.from(plaintext).toString();
    const newAccounts = JSON.parse(decodedPlaintext) as Accounts;

    // Generate salt and use it to derive encryption key
    const newSalt = randomBytes(pbkdf2SaltSize);
    const newEncryptionKey = await deriveEncryptionKey(newPassword, newSalt);

    // Initialize new encrypted state
    const newPlaintext = JSON.stringify(newAccounts);
    const newNonce = randomBytes(secretbox.nonceLength);
    const newCiphertext = secretbox(Buffer.from(newPlaintext), newNonce, newEncryptionKey);

    const newEncryptedAccounts = {
      ciphertext: bs58.encode(newCiphertext),
      nonce: bs58.encode(newNonce),
    };

    await Promise.all([
      updatePersistentState({
        encryptedAccounts: newEncryptedAccounts,
        salt: bs58.encode(newSalt),
      }),
      updateSessionState({
        encryptionKey: bs58.encode(newEncryptionKey),
      }),
    ]);
  };

  return {
    changePassword,
    clearAccounts,
    encryptedAccounts,
    lockAccounts,
    unlockAccounts,
  };
});

export interface UseUnlockedAccountsProps {
  accounts: Accounts,
  encryptionKey: string,
}

/**
 * Hook for accessing and updating the accounts state.
 * Requires the accounts state to be unlocked
 */
export const [UnlockedAccountsProvider, useUnlockedAccounts] = constate(({
  accounts,
  encryptionKey,
}: UseUnlockedAccountsProps) => {
  const {
    activeAccountAddress,
    updatePersistentState,
    updateSessionState,
  } = useAppState();

  const encryptAccounts = (newAccounts: Accounts) => {
    const plaintext = JSON.stringify(newAccounts);
    const nonce = randomBytes(secretbox.nonceLength);
    const ciphertext = secretbox(Buffer.from(plaintext), nonce, bs58.decode(encryptionKey));
    return {
      ciphertext: bs58.encode(ciphertext),
      nonce: bs58.encode(nonce),
    } as EncryptedAccounts;
  };

  const addAccount = async (account: Account) => {
    const newAccounts = { ...accounts, [account.address]: account };
    const newEncryptedAccounts = encryptAccounts(newAccounts);

    await updateSessionState({ accounts: newAccounts });
    await updatePersistentState({
      activeAccountAddress: account.address,
      activeAccountPublicKey: account.publicKey,
      encryptedAccounts: newEncryptedAccounts,
    });
    const publicAccount = {
      address: account.address,
      publicKey: account.publicKey,
    };
    await triggerAccountChange(publicAccount);
  };

  const removeAccount = async (address: string) => {
    const newAccounts = { ...accounts };
    delete newAccounts[address];
    const newEncryptedAccounts = encryptAccounts(newAccounts);

    // Switch account to first available when deleting the active account
    if (address === activeAccountAddress) {
      const firstAvailableAddress = Object.keys(newAccounts)[0];
      const firstAvailableAccount = firstAvailableAddress !== undefined
        ? newAccounts[firstAvailableAddress]
        : undefined;
      // Note: need to await update to `activeAccountAddress` before `accounts`
      await updatePersistentState({
        activeAccountAddress: firstAvailableAccount?.address,
        activeAccountPublicKey: firstAvailableAccount?.publicKey,
        encryptedAccounts: newEncryptedAccounts,
      });

      const publicAccount = firstAvailableAccount !== undefined
        ? {
          address: firstAvailableAccount.address,
          publicKey: firstAvailableAccount.publicKey,
        }
        : undefined;
      await triggerAccountChange(publicAccount);
    } else {
      await updatePersistentState({ encryptedAccounts: newEncryptedAccounts });
    }

    await updateSessionState({ accounts: newAccounts });
  };

  const renameAccount = async (address: string, newName: string) => {
    if (address in accounts) {
      const newAccounts = { ...accounts };
      newAccounts[address] = { ...newAccounts[address], name: newName };
      const newEncryptedAccounts = encryptAccounts(newAccounts);

      await Promise.all([
        updatePersistentState({ encryptedAccounts: newEncryptedAccounts }),
        updateSessionState({ accounts: newAccounts }),
      ]);
    }
  };

  const switchAccount = async (address: string) => {
    const publicKey = accounts[address]?.publicKey;
    const publicAccount = address !== undefined && publicKey !== undefined
      ? { address, publicKey }
      : undefined;

    await updatePersistentState({
      activeAccountAddress: address,
      activeAccountPublicKey: publicAccount?.publicKey,
    });
    await triggerAccountChange(publicAccount);
  };

  return {
    accounts,
    addAccount,
    removeAccount,
    renameAccount,
    switchAccount,
  };
});

export interface UseActiveAccountProps {
  activeAccountAddress: string,
}

/**
 * Hook for accessing the active account.
 * Requires the accounts state to be unlocked and have at least an account
 */
export const [ActiveAccountProvider, useActiveAccount] = constate(({
  activeAccountAddress,
}: UseActiveAccountProps) => {
  const { accounts } = useUnlockedAccounts();
  const activeAccount = accounts[activeAccountAddress];

  const aptosAccount = new AptosAccount(
    HexString.ensure(activeAccount.privateKey).toUint8Array(),
    activeAccount.address,
  );

  return {
    activeAccount,
    activeAccountAddress,
    aptosAccount,
  };
});
