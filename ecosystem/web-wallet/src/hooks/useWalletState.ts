// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { useState, useCallback } from 'react'
import constate from 'constate'
import { AptosAccount, AptosAccountObject } from 'aptos'

export type AptosAccountState = AptosAccount | undefined;

export interface LocalStorageState {
  aptosAccountObject?: AptosAccountObject,
}

export interface UpdateWalletStateProps {
  aptosAccountState: AptosAccountState
}

const defaultValue: LocalStorageState = {
  aptosAccountObject: undefined
}

const walletStateLocalStorageKey = 'aptosWalletState'

export default function useWalletState () {
  const [localStorageState, setLocalStorageState] = useState<LocalStorageState>(() => {
    if (typeof window === 'undefined') {
      return defaultValue
    }
    try {
      // Get from local storage by key
      const item = window.localStorage.getItem(walletStateLocalStorageKey)
      const result: LocalStorageState = item ? JSON.parse(item) : defaultValue
      return result
    } catch (error) {
      return defaultValue
    }
  })

  const [aptosAccount, setAptosAccount] = useState<AptosAccountState>(() => {
    if (typeof window === 'undefined') {
      return undefined
    }
    try {
      const item = window.localStorage.getItem(walletStateLocalStorageKey)
      const localStorageState: AptosAccountObject = item ? JSON.parse(item) : defaultValue
      if (localStorageState) {
        const aptosAccount = AptosAccount.fromAptosAccountObject(localStorageState)
        return aptosAccount
      } else {
        console.log('Unable to retrieve from local storage')
        return undefined
      }
    } catch (err) {
      console.log(err)
      return undefined
    }
  })

  const updateWalletState = useCallback(({ aptosAccountState }: UpdateWalletStateProps) => {
    if (typeof window === 'undefined') {
      return
    }
    try {
      const privateKeyObject = aptosAccountState?.toPrivateKeyObject()
      setAptosAccount(aptosAccountState)
      setLocalStorageState({ aptosAccountObject: privateKeyObject })
      window.localStorage.setItem(walletStateLocalStorageKey, JSON.stringify(privateKeyObject))
    } catch (error) {
      console.log(error)
    }
  }, [])

  return { walletState: localStorageState, aptosAccount, updateWalletState }
}

export const [WalletStateProvider, useWalletStateContext] = constate(useWalletState)
