/* eslint-disable no-console */
// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { useState, useCallback, useMemo } from 'react';
import constate from 'constate';
import { getLocalStorageState } from 'core/utils/account';
import { WALLET_STATE_LOCAL_STORAGE_KEY, WALLET_STATE_NETWORK_LOCAL_STORAGE_KEY } from 'core/constants';
import { AptosAccountState, LocalStorageState } from 'core/types/stateTypes';
import {
  AptosNetwork, getFaucetNetworkFromAptosNetwork, getLocalStorageNetworkState,
} from 'core/utils/network';
import Browser from 'core/utils/browser';
import { AptosAccount, FaucetClient } from 'aptos';
import { toast } from 'core/components/Toast';

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

  const addAccount = useCallback(async ({
    account,
  }: AddAccountProps) => {
    const faucetClient = new FaucetClient(aptosNetwork, faucetNetwork);
    let localStorageStateCopy = { ...localStorageState };
    localStorageStateCopy = {
      aptosAccounts: {
        ...localStorageStateCopy.aptosAccounts,
        [account.address().hex()]: account.toPrivateKeyObject(),
      },
      currAccountAddress: account.address().hex(),
    };
    try {
      await faucetClient.fundAccount(account.address(), 0);
      setLocalStorageState(localStorageStateCopy);
      window.localStorage.setItem(
        WALLET_STATE_LOCAL_STORAGE_KEY,
        JSON.stringify(localStorageStateCopy),
      );
      Browser.storage()?.set({ [WALLET_STATE_LOCAL_STORAGE_KEY]: localStorageStateCopy });
      toast({
        description: 'Successfully created new account',
        duration: 5000,
        isClosable: true,
        status: 'success',
        title: 'Created account',
        variant: 'solid',
      });
    } catch (err) {
      toast({
        description: 'Error creating new account',
        duration: 5000,
        isClosable: true,
        status: 'error',
        title: 'Error creating account',
        variant: 'solid',
      });
      console.error(err);
    }
  }, []);

  const switchAccount = useCallback(({ accountAddress }: RemoveAccountProps) => {
    if (!accountAddress
      || (localStorageState.aptosAccounts
         && localStorageState.aptosAccounts[accountAddress] === undefined)
    ) {
      console.error('No account found');
      return;
    }
    const localStorageStateCopy = {
      ...localStorageState,
      currAccountAddress: accountAddress,
    };
    try {
      setLocalStorageState(localStorageStateCopy);
      window.localStorage.setItem(
        WALLET_STATE_LOCAL_STORAGE_KEY,
        JSON.stringify(localStorageStateCopy),
      );
      Browser.storage()?.set({ [WALLET_STATE_LOCAL_STORAGE_KEY]: localStorageStateCopy });
      toast({
        description: `Successfully switched account to ${accountAddress.substring(0, 6)}...`,
        duration: 5000,
        isClosable: true,
        status: 'success',
        title: 'Switched account',
        variant: 'solid',
      });
    } catch (error) {
      toast({
        description: 'Error during account switch',
        duration: 5000,
        isClosable: true,
        status: 'error',
        title: 'Error switch account',
        variant: 'solid',
      });
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
    let newAccountAddress: string | undefined;
    let toastMessage = `Still using account with address: ${accountAddress?.substring(0, 6)}...`;
    let localStorageStateCopy = { ...localStorageState };
    if (
      !accountAddress
      || !localStorageStateCopy.aptosAccounts
      || localStorageStateCopy.aptosAccounts[accountAddress] === undefined
    ) {
      console.error('No account found');
      return;
    }
    delete localStorageStateCopy.aptosAccounts[accountAddress];

    if (Object.keys(localStorageStateCopy.aptosAccounts).length === 0) {
      newAccountAddress = undefined;
      toastMessage = 'No other accounts in wallet, signing out';
    } else if (accountAddress === currAccountAddress) {
      // switch to another account in wallet
      if (Object.keys(localStorageStateCopy.aptosAccounts).length >= 1) {
        newAccountAddress = localStorageStateCopy.aptosAccounts[
          Object.keys(localStorageStateCopy.aptosAccounts)[0]
        ].address;
      }
      toastMessage = `Switching to account with address: ${newAccountAddress?.substring(0, 6)}...`;
    } else {
      newAccountAddress = currAccountAddress;
      toastMessage = `Using the same account with address: ${newAccountAddress?.substring(0, 6)}...`;
    }

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
      toast({
        description: toastMessage,
        duration: 5000,
        isClosable: true,
        status: 'success',
        title: 'Deleted account',
        variant: 'solid',
      });
    } catch (err) {
      toast({
        description: 'Account deletion process incurred an error',
        duration: 5000,
        isClosable: true,
        status: 'error',
        title: 'Error deleting account',
        variant: 'solid',
      });
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
