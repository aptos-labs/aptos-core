/* eslint-disable no-console */
// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { useState, useCallback, useMemo } from 'react';
import constate from 'constate';
import { getLocalStorageState } from 'core/utils/account';
import { WALLET_STATE_LOCAL_STORAGE_KEY, WALLET_STATE_NETWORK_LOCAL_STORAGE_KEY } from 'core/constants';
import { AptosAccountState, LocalStorageState } from 'core/types';
import {
  AptosNetwork, getFaucetNetworkFromAptosNetwork, getLocalStorageNetworkState,
} from 'core/utils/network';
import Browser from 'core/utils/browser';
import { AptosAccount } from 'aptos';

const defaultValue: LocalStorageState = {
  aptosAccounts: undefined,
  currAccountAddress: undefined,
};

export interface UpdateWalletStateProps {
  account: AptosAccountState
}

export interface AddAccountProps {
  account: AptosAccount
}

export interface RemoveAccountProps {
  accountAddress?: string;
}

export default function useWalletState() {
  const [localStorageState, setLocalStorageState] = useState<LocalStorageState>(
    () => getLocalStorageState() ?? defaultValue,
  );

  const { currAccountAddress } = localStorageState;

  const aptosAccount = (localStorageState.aptosAccounts && currAccountAddress)
    ? AptosAccount.fromAptosAccountObject(
      localStorageState.aptosAccounts[currAccountAddress],
    ) : undefined;

  const [aptosNetwork, setAptosNetwork] = useState<AptosNetwork>(
    () => getLocalStorageNetworkState(),
  );

  const faucetNetwork = useMemo(
    () => getFaucetNetworkFromAptosNetwork(aptosNetwork),
    [aptosNetwork],
  );

  const addAccount = useCallback(({
    account,
  }: AddAccountProps) => {
    let localStorageStateCopy = { ...localStorageState };
    localStorageStateCopy = {
      aptosAccounts: {
        ...localStorageStateCopy.aptosAccounts,
        [account.address().hex()]: account.toPrivateKeyObject(),
      },
      currAccountAddress: account.address().hex(),
    };
    try {
      setLocalStorageState(localStorageStateCopy);
      window.localStorage.setItem(
        WALLET_STATE_LOCAL_STORAGE_KEY,
        JSON.stringify(localStorageStateCopy),
      );
      Browser.storage()?.set({ [WALLET_STATE_LOCAL_STORAGE_KEY]: localStorageStateCopy });
    } catch (err) {
      console.error(err);
    }
  }, []);

  const switchAccount = useCallback(({ account }: UpdateWalletStateProps) => {
    if (!account) {
      console.error('No account found');
      return;
    }
    const localStorageStateCopy = {
      ...localStorageState,
      currAccountAddress: account.address().hex(),
    };
    try {
      setLocalStorageState(localStorageStateCopy);
      window.localStorage.setItem(
        WALLET_STATE_LOCAL_STORAGE_KEY,
        JSON.stringify(localStorageStateCopy),
      );
      Browser.storage()?.set({ [WALLET_STATE_LOCAL_STORAGE_KEY]: localStorageStateCopy });
    } catch (error) {
      console.error(error);
    }
  }, []);

  const updateNetworkState = useCallback((network: AptosNetwork) => {
    try {
      setAptosNetwork(network);
      window.localStorage.setItem(WALLET_STATE_NETWORK_LOCAL_STORAGE_KEY, network);
    } catch (error) {
      console.error(error);
    }
  }, []);

  const removeAccount = useCallback(({
    accountAddress,
  }: RemoveAccountProps) => {
    let localStorageStateCopy = { ...localStorageState };
    if (!accountAddress || !localStorageStateCopy.aptosAccounts) {
      console.error('No account found');
      return;
    }
    delete localStorageStateCopy.aptosAccounts[accountAddress];
    localStorageStateCopy = {
      ...localStorageStateCopy,
      currAccountAddress: undefined,
    };
    try {
      setLocalStorageState(localStorageStateCopy);
      window.localStorage.setItem(
        WALLET_STATE_LOCAL_STORAGE_KEY,
        JSON.stringify(localStorageStateCopy),
      );
      Browser.storage()?.set({ [WALLET_STATE_LOCAL_STORAGE_KEY]: localStorageStateCopy });
    } catch (err) {
      console.error(err);
    }
  }, []);

  return {
    addAccount,
    aptosAccount,
    aptosNetwork,
    currAccountAddress,
    faucetNetwork,
    removeAccount,
    switchAccount,
    updateNetworkState,
    walletState: localStorageState,
  };
}

export const [WalletStateProvider, useWalletStateContext] = constate(useWalletState);
