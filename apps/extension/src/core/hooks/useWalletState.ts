/* eslint-disable no-console */
// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { useState, useCallback, useMemo } from 'react';
import constate from 'constate';
import { getLocalStorageState } from 'core/utils/account';
import { WALLET_STATE_LOCAL_STORAGE_KEY, WALLET_STATE_NETWORK_LOCAL_STORAGE_KEY } from 'core/constants';
import {
  AptosAccountState, LocalStorageState, Mnemonic, WalletAccount,
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

const defaultValue: LocalStorageState = {
  accounts: null,
  currAccountAddress: null,
};

export interface UpdateWalletStateProps {
  account: AptosAccountState
}

export interface AddAccountProps {
  account: AptosAccount
  isImport?: boolean;
  mnemonic?: Mnemonic
}

export interface RemoveAccountProps {
  accountAddress?: string;
}

export default function useWalletState() {
  const [localStorageState, setLocalStorageState] = useState<LocalStorageState>(
    () => getLocalStorageState() ?? defaultValue,
  );

  const { currAccountAddress } = localStorageState;

  const aptosAccount = (localStorageState.accounts && currAccountAddress)
    ? AptosAccount.fromAptosAccountObject(
      localStorageState.accounts[currAccountAddress].aptosAccount,
    ) : undefined;

  const accountMnemonic = (localStorageState.accounts && currAccountAddress)
    ? localStorageState.accounts[currAccountAddress].mnemonic
    : undefined;

  const [nodeUrl, setNodeUrl] = useState<NodeUrl>(
    () => getLocalStorageNodeNetworkUrl(),
  );

  const faucetNetwork = useMemo(
    () => getFaucetUrlFromNodeUrl(nodeUrl),
    [nodeUrl],
  );

  const addAccount = useCallback(async ({
    account, isImport = false, mnemonic,
  }: AddAccountProps) => {
    const newAccountAddress = account.address().hex();
    console.log('hi');

    // check if account already exists
    if (localStorageState.accounts !== null && newAccountAddress in localStorageState.accounts) {
      importAccountErrorAccountAlreadyExistsToast();
      throw new Error('Account already exists');
    }
    const newAccount: WalletAccount = {
      aptosAccount: account.toPrivateKeyObject(),
      mnemonic,
    };
    let localStorageStateCopy = { ...localStorageState };
    localStorageStateCopy = {
      accounts: {
        ...localStorageStateCopy.accounts,
        [newAccountAddress]: newAccount,
      },
      currAccountAddress: newAccountAddress,
    };
    try {
      if (faucetNetwork) {
        const faucetClient = new FaucetClient(nodeUrl, faucetNetwork);
        await faucetClient.fundAccount(account.address(), 0);
      }
      setLocalStorageState(localStorageStateCopy);
      const localStorageStateString = JSON.stringify(localStorageStateCopy);
      window.localStorage.setItem(
        WALLET_STATE_LOCAL_STORAGE_KEY,
        localStorageStateString,
      );
      Browser.storage()?.set({ [WALLET_STATE_LOCAL_STORAGE_KEY]: localStorageStateString });
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
  }, [nodeUrl, faucetNetwork, localStorageState]);

  const switchAccount = useCallback(({ accountAddress }: RemoveAccountProps) => {
    if (!accountAddress
      || (localStorageState.accounts
         && localStorageState.accounts[accountAddress] === undefined)
    ) {
      console.error('No account found');
      return;
    }
    const account = AptosAccount.fromAptosAccountObject(
      localStorageState.accounts![accountAddress].aptosAccount,
    );
    const localStorageStateCopy = {
      ...localStorageState,
      currAccountAddress: accountAddress,
    };
    try {
      setLocalStorageState(localStorageStateCopy);
      const localStorageStateString = JSON.stringify(localStorageStateCopy);
      window.localStorage.setItem(
        WALLET_STATE_LOCAL_STORAGE_KEY,
        localStorageStateString,
      );
      Browser.storage()?.set({ [WALLET_STATE_LOCAL_STORAGE_KEY]: localStorageStateString });
      switchAccountToast(accountAddress);
      sendProviderEvent(ProviderEvent.ACCOUNT_CHANGED, account);
    } catch (error) {
      switchAccountErrorToast();
      console.error(error);
    }
  }, [localStorageState]);

  const updateNetworkState = useCallback((network: NodeUrl) => {
    try {
      setNodeUrl(network);
      window.localStorage.setItem(WALLET_STATE_NETWORK_LOCAL_STORAGE_KEY, network);
      Browser.storage()?.set({ [WALLET_STATE_NETWORK_LOCAL_STORAGE_KEY]: network });
      sendProviderEvent(ProviderEvent.NETWORK_CHANGED, aptosAccount);
    } catch (error) {
      console.error(error);
    }
  }, [aptosAccount]);

  const removeAccount = useCallback(({
    accountAddress,
  }: RemoveAccountProps) => {
    let newAccountAddress: string | null = null;
    let toastMessage = `Still using account with address: ${accountAddress?.substring(0, 6)}...`;
    let localStorageStateCopy: LocalStorageState = { ...localStorageState };
    if (
      !accountAddress
      || !localStorageStateCopy.accounts
      || localStorageStateCopy.accounts[accountAddress] === undefined
    ) {
      console.error('No account found');
      return;
    }
    delete localStorageStateCopy.accounts[accountAddress];

    if (Object.keys(localStorageStateCopy.accounts).length === 0) {
      newAccountAddress = null;
      toastMessage = 'No other accounts in wallet, signing out';
    } else if (accountAddress === currAccountAddress) {
      // switch to another account in wallet
      if (Object.keys(localStorageStateCopy.accounts).length >= 1) {
        newAccountAddress = localStorageStateCopy.accounts[
          Object.keys(localStorageStateCopy.accounts)[0]
        ].aptosAccount.address!;
      }
      toastMessage = `Switching to account with address: ${newAccountAddress?.substring(0, 6)}...`;
    } else {
      newAccountAddress = currAccountAddress || null;
      toastMessage = `Using the same account with address: ${newAccountAddress?.substring(0, 6)}...`;
    }
    console.log('hi');
    localStorageStateCopy = {
      ...localStorageStateCopy,
      currAccountAddress: newAccountAddress,
    };
    try {
      setLocalStorageState(localStorageStateCopy);
      window.localStorage.setItem(
        WALLET_STATE_LOCAL_STORAGE_KEY,
        JSON.stringify(localStorageStateCopy),
      );
      Browser.storage()?.set({ [WALLET_STATE_LOCAL_STORAGE_KEY]: localStorageStateCopy });
      removeAccountToast(toastMessage);
    } catch (err) {
      removeAccountErrorToast();
      console.error(err);
    }
  }, [currAccountAddress, localStorageState]);

  return {
    accountMnemonic,
    addAccount,
    aptosAccount,
    currAccountAddress,
    faucetNetwork,
    nodeUrl,
    removeAccount,
    switchAccount,
    updateNetworkState,
    walletState: localStorageState,
  };
}

export const [WalletStateProvider, useWalletStateContext] = constate(useWalletState);
