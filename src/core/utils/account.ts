// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { AptosAccount, AptosAccountObject } from 'aptos';
import { WALLET_STATE_LOCAL_STORAGE_KEY } from 'core/constants';
import { AptosAccountState, LocalStorageState } from 'core/types';

import Browser from 'core/utils/browser';

export function createNewAccount(): AptosAccount {
  const account = new AptosAccount();
  // todo: make request to create account on chain
  return account;
}

export function getLocalStorageState(): LocalStorageState | null {
  // Get from local storage by key
  const item = window.localStorage.getItem(WALLET_STATE_LOCAL_STORAGE_KEY);
  if (item) {
    const accountObject: AptosAccountObject = JSON.parse(item);
    return { aptosAccountObject: accountObject };
  }
  return null;
}

export function getAptosAccountState(): AptosAccountState {
  const localStorage = getLocalStorageState();
  if (localStorage) {
    const { aptosAccountObject } = localStorage;
    return aptosAccountObject ? AptosAccount.fromAptosAccountObject(aptosAccountObject) : undefined;
  }
  return undefined;
}

export function getBackgroundAptosAccountState(): Promise<AptosAccountState> {
  return new Promise((resolve) => {
    Browser.storage()?.get([WALLET_STATE_LOCAL_STORAGE_KEY], (result: any) => {
      const aptosAccountObject: AptosAccountObject = result[WALLET_STATE_LOCAL_STORAGE_KEY];
      resolve(aptosAccountObject
        ? AptosAccount.fromAptosAccountObject(aptosAccountObject)
        : undefined);
    });
  });
}
