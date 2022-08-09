/* eslint-disable no-console */
// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { useState, useCallback, useMemo } from 'react';
import constate from 'constate';
import {
  getDecryptedAccounts, getCurrentPublicAccount, storeEncryptedAccounts,
} from 'core/utils/account';
import {
  WALLET_STATE_ACCOUNT_ADDRESS_KEY,
  WALLET_STATE_NETWORK_LOCAL_STORAGE_KEY,
} from 'core/constants';
import {
  AccountsState,
  AptosAccountState,
  Mnemonic,
  PublicAccount,
  WalletAccount,
} from 'core/types/stateTypes';
import {
  getFaucetUrlFromNodeUrl, getLocalStorageNodeNetworkUrl, NodeUrl,
} from 'core/utils/network';
import Browser from 'core/utils/browser';
import { AptosAccount, FaucetClient } from 'aptos';
import {
  createAccountToast,
  createAccountErrorToast,
  switchAccountToast,
  switchAccountErrorToast,
  removeAccountToast,
  removeAccountErrorToast,
  importAccountErrorToast,
  importAccountToast,
  importAccountErrorAccountAlreadyExistsToast,
} from 'core/components/Toast';
import { ProviderEvent, sendProviderEvent } from 'core/utils/providerEvents';

export interface UpdateWalletStateProps {
  account: AptosAccountState
}

export interface AddAccountProps {
  account: AptosAccount
  isImport?: boolean
  mnemonic?: Mnemonic
  password?: string
}

export interface RemoveAccountProps {
  accountAddress?: string;
}

export default function useWalletStateRecorder() {
  const [currAccountAddress, setCurrAccountAddress] = useState<string | undefined>(
    getCurrentPublicAccount()?.address,
  );

  const [activeAccounts, setActiveAccounts] = useState<AccountsState | null>(
    () => getDecryptedAccounts(),
  );

  const updateCurrentAccount = (publicAccount: PublicAccount | null) => {
    setCurrAccountAddress(publicAccount?.address ?? undefined);
    const string = JSON.stringify(publicAccount);
    if (publicAccount) {
      window.localStorage.setItem(WALLET_STATE_ACCOUNT_ADDRESS_KEY, string);
    } else {
      window.localStorage.removeItem(WALLET_STATE_ACCOUNT_ADDRESS_KEY);
    }
    Browser.persistentStorage()?.set({ [WALLET_STATE_ACCOUNT_ADDRESS_KEY]: string });
  };

  const updateAccountsState = async (
    accounts: AccountsState,
    password: string | undefined = undefined,
  ) => {
    setActiveAccounts(accounts);
    await storeEncryptedAccounts(accounts, password);
  };

  const aptosAccount = (activeAccounts && currAccountAddress)
    ? AptosAccount.fromAptosAccountObject(
      activeAccounts[currAccountAddress].aptosAccount,
    ) : undefined;

  const accountMnemonic = (activeAccounts && currAccountAddress)
    ? activeAccounts[currAccountAddress].mnemonic
    : undefined;

  const [nodeUrl, setNodeUrl] = useState<NodeUrl>(
    () => getLocalStorageNodeNetworkUrl(),
  );

  const faucetNetwork = useMemo(
    () => getFaucetUrlFromNodeUrl(nodeUrl),
    [nodeUrl],
  );

  const addAccount = useCallback(async ({
    account, isImport = false, mnemonic, password = undefined,
  }: AddAccountProps) => {
    const newAccountAddress = account.address().hex();

    // check if account already exists
    if (activeAccounts
      && activeAccounts.accounts
      && newAccountAddress in activeAccounts.accounts) {
      importAccountErrorAccountAlreadyExistsToast();
      throw new Error('Account already exists');
    }
    const newAccount: WalletAccount = {
      aptosAccount: account.toPrivateKeyObject(),
      mnemonic,
    };
    try {
      if (faucetNetwork) {
        const faucetClient = new FaucetClient(nodeUrl, faucetNetwork);
        await faucetClient.fundAccount(account.address(), 0);
      }
      await updateAccountsState({
        ...activeAccounts,
        [account.address().hex()]: newAccount,
      }, password);
      updateCurrentAccount({
        address: account.address().hex(),
        publicKey: account.pubKey().hex(),
      });
      sendProviderEvent(ProviderEvent.ACCOUNT_CHANGED, account);
      if (isImport) {
        importAccountToast();
      } else {
        createAccountToast();
      }
    } catch (err) {
      if (isImport) {
        importAccountErrorToast();
      } else {
        createAccountErrorToast();
      }
      console.error(err);
      throw err;
    }
  }, [nodeUrl, faucetNetwork, activeAccounts]);

  const switchAccount = useCallback(({ accountAddress }: RemoveAccountProps) => {
    if (!accountAddress
      || (activeAccounts
         && activeAccounts[accountAddress] === undefined)
    ) {
      console.error('No account found');
      return;
    }
    const account = AptosAccount.fromAptosAccountObject(
      activeAccounts![accountAddress].aptosAccount,
    );
    try {
      updateCurrentAccount({
        address: accountAddress,
        publicKey: account.pubKey().hex(),
      });
      switchAccountToast(accountAddress);
      sendProviderEvent(ProviderEvent.ACCOUNT_CHANGED, account);
    } catch (error) {
      switchAccountErrorToast();
      console.error(error);
    }
  }, [activeAccounts]);

  const updateNetworkState = useCallback((network: NodeUrl) => {
    try {
      setNodeUrl(network);
      window.localStorage.setItem(WALLET_STATE_NETWORK_LOCAL_STORAGE_KEY, network);
      Browser.persistentStorage()?.set({ [WALLET_STATE_NETWORK_LOCAL_STORAGE_KEY]: network });
      sendProviderEvent(ProviderEvent.NETWORK_CHANGED, aptosAccount);
    } catch (error) {
      console.error(error);
    }
  }, [aptosAccount]);

  const removeAccount = useCallback(async ({
    accountAddress,
  }: RemoveAccountProps) => {
    let newAccountAddress: string | null = null;
    let toastMessage = `Still using account with address: ${accountAddress?.substring(0, 6)}...`;
    if (
      !accountAddress
      || !activeAccounts
      || activeAccounts[accountAddress] === undefined
    ) {
      console.error('No account found');
      return;
    }
    delete activeAccounts[accountAddress];

    if (Object.keys(activeAccounts).length === 0) {
      newAccountAddress = null;
      toastMessage = 'No other accounts in wallet, signing out';
    } else if (accountAddress === currAccountAddress) {
      // switch to another account in wallet
      if (Object.keys(activeAccounts).length >= 1) {
        newAccountAddress = activeAccounts[
          Object.keys(activeAccounts)[0]
        ].aptosAccount.address!;
      }
      toastMessage = `Switching to account with address: ${newAccountAddress?.substring(0, 6)}...`;
    } else {
      newAccountAddress = currAccountAddress || null;
      toastMessage = `Using the same account with address: ${newAccountAddress?.substring(0, 6)}...`;
    }
    try {
      const account = activeAccounts && newAccountAddress ? AptosAccount.fromAptosAccountObject(
        activeAccounts[newAccountAddress].aptosAccount,
      ) : null;
      await updateAccountsState(activeAccounts);
      updateCurrentAccount(account ? {
        address: account.address().hex(),
        publicKey: account.pubKey().hex(),
      } : null);
      removeAccountToast(toastMessage);
    } catch (err) {
      removeAccountErrorToast();
      console.error(err);
    }
  }, [activeAccounts, currAccountAddress]);

  return {
    accountMnemonic,
    accounts: activeAccounts,
    addAccount,
    aptosAccount,
    currAccountAddress,
    faucetNetwork,
    nodeUrl,
    removeAccount,
    switchAccount,
    updateNetworkState,
  };
}

export const [WalletStateProvider, useWalletState] = constate(useWalletStateRecorder);
