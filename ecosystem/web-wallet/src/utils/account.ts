// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { AptosAccount, AptosAccountObject } from 'aptos'
import { Buffer } from 'buffer'
import { KEY_LENGTH, WALLET_STATE_LOCAL_STORAGE_KEY } from '../constants'
import { AptosAccountState, LocalStorageState, Result, err, ok } from '../types'

export function loginAccount (key: string): Result<AptosAccount, Error> {
  if (key.length === KEY_LENGTH) {
    try {
      const encodedKey = Uint8Array.from(Buffer.from(key, 'hex'))
      // todo: Ping API to check if a legit account
      const account = new AptosAccount(encodedKey, undefined)
      return ok(account)
    } catch (e) {
      return err(e as Error)
    }
  } else {
    return err(new Error('Key not the correct the length'))
  }
}

export function createNewAccount (): AptosAccount {
  const account = new AptosAccount()
  // todo: make request to create account on chain
  return account
}

export function getLocalStorageState (): LocalStorageState | null {
  // Get from local storage by key
  const item = window.localStorage.getItem(WALLET_STATE_LOCAL_STORAGE_KEY)
  if (item) {
    const accountObject: AptosAccountObject = JSON.parse(item)
    return { aptosAccountObject: accountObject }
  } else {
    return null
  }
}

export function getAptosAccountState (): AptosAccountState {
  const localStorage = getLocalStorageState()
  if (localStorage) {
    const { aptosAccountObject } = localStorage
    return aptosAccountObject ? AptosAccount.fromAptosAccountObject(aptosAccountObject) : undefined
  } else {
    return undefined
  }
}
