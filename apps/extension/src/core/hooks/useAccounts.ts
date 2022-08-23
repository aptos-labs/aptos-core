// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { usePersistentStorageState } from 'core/hooks/useStorageState';
import useEncryptedAccounts from 'core/hooks/useEncryptedStorageState';
import { AptosAccount, HexString } from 'aptos';
import { Account, PublicAccount } from 'core/types/stateTypes';
import { WALLET_STATE_ACCOUNT_ADDRESS_KEY, WALLET_STATE_STYLE_INDEX_KEY } from 'core/constants';
import { ProviderEvent, sendProviderEvent } from 'core/utils/providerEvents';

/**
 * Hook for managing wallet accounts
 */
export default function useAccounts() {
  const {
    clear,
    initialize: initEncryptedState,
    isInitialized: areAccountsInitialized,
    isReady: isEncryptedStateReady,
    isUnlocked: areAccountsUnlocked,
    lock: lockAccounts,
    unlock: unlockAccounts,
    update,
    value: accounts,
  } = useEncryptedAccounts();

  const [
    activePublicAccount,
    setActivePublicAccount,
    isActivePublicAccountReady,
  ] = usePersistentStorageState<PublicAccount>(WALLET_STATE_ACCOUNT_ADDRESS_KEY);

  const [
    newAccountStyleIndex,
    setNewAccountStyleIndex,
    isNewAccountStyleIndexReady,
  ] = usePersistentStorageState<number>(WALLET_STATE_STYLE_INDEX_KEY);

  const areAccountsReady = (isEncryptedStateReady
                            && isActivePublicAccountReady
                            && isNewAccountStyleIndexReady);
  const activeAccountAddress = activePublicAccount?.address;

  const activeAccount = accounts && activeAccountAddress
    ? accounts[activeAccountAddress]
    : undefined;

  const initAccounts = async (password: string, firstAccount: Account) => {
    setNewAccountStyleIndex(1);
    await setActivePublicAccount({
      address: firstAccount.address,
      publicKey: firstAccount.publicKey,
    });
    await initEncryptedState(password, { [firstAccount.address]: firstAccount });
    await sendProviderEvent(ProviderEvent.ACCOUNT_CHANGED);
  };

  const addAccount = async (account: Account) => {
    await setNewAccountStyleIndex((newAccountStyleIndex ?? 0) + 1);
    const newAccounts = { ...accounts!, [account.address]: account };
    await update(newAccounts);
    await setActivePublicAccount({
      address: account.address,
      publicKey: account.publicKey,
    });
    await sendProviderEvent(ProviderEvent.ACCOUNT_CHANGED);
  };

  const removeAccount = async (address: string) => {
    const newAccounts = { ...accounts! };
    delete newAccounts[address];
    await update(newAccounts);

    if (address === activeAccountAddress) {
      const firstAvailableAddress = Object.keys(newAccounts)[0];
      const firstAvailableAccount = newAccounts[firstAvailableAddress];
      await setActivePublicAccount(firstAvailableAccount ? {
        address: firstAvailableAccount.address,
        publicKey: firstAvailableAccount.publicKey,
      } : undefined);
      await sendProviderEvent(ProviderEvent.ACCOUNT_CHANGED);
    }
  };

  const switchAccount = async (address: string) => {
    if (address in accounts!) {
      const account = accounts![address];
      await setActivePublicAccount({ address, publicKey: account.publicKey });
      await sendProviderEvent(ProviderEvent.ACCOUNT_CHANGED);
    }
  };

  const renameAccount = async (address: string, newName: string) => {
    if (address in accounts!) {
      const newAccounts = { ...accounts! };
      newAccounts[address] = { ...newAccounts[address], name: newName };
      await update(newAccounts);
    }
  };

  const resetAccount = async () => {
    await setActivePublicAccount(undefined);
    await clear();
  };

  const aptosAccount = activeAccount ? new AptosAccount(
    HexString.ensure(activeAccount.privateKey).toUint8Array(),
    activeAccount.address,
  ) : undefined;

  return {
    accounts,
    activeAccount,
    activeAccountAddress,
    addAccount,
    aptosAccount,
    areAccountsInitialized,
    areAccountsReady,
    areAccountsUnlocked,
    initAccounts,
    lockAccounts,
    newAccountStyleIndex,
    removeAccount,
    renameAccount,
    resetAccount,
    switchAccount,
    unlockAccounts,
  };
}
