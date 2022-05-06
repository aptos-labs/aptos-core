// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { useState, useCallback } from 'react'
import constate from 'constate'
import { getAptosAccountState, getLocalStorageState } from '../utils/account'
import { WALLET_STATE_LOCAL_STORAGE_KEY } from '../constants'
import { AptosAccountState, LocalStorageState } from '../types'

const defaultValue: LocalStorageState = {
  aptosAccountObject: undefined
}

export interface UpdateWalletStateProps {
  aptosAccountState: AptosAccountState
}

export default function useWalletState () {
  const [localStorageState, setLocalStorageState] = useState<LocalStorageState>(() => {
    return getLocalStorageState() ?? defaultValue
  })

  const [aptosAccount, setAptosAccount] = useState<AptosAccountState>(() => {
    return getAptosAccountState()
  })

  const updateWalletState = useCallback(({ aptosAccountState }: UpdateWalletStateProps) => {
    try {
      const privateKeyObject = aptosAccountState?.toPrivateKeyObject()
      setAptosAccount(aptosAccountState)
      setLocalStorageState({ aptosAccountObject: privateKeyObject })
      window.localStorage.setItem(WALLET_STATE_LOCAL_STORAGE_KEY, JSON.stringify(privateKeyObject))
    } catch (error) {
      console.log(error)
    }
  }, [])

  const signOut = useCallback(() => {
    setAptosAccount(undefined)
    setLocalStorageState({ aptosAccountObject: undefined })
    window.localStorage.removeItem(WALLET_STATE_LOCAL_STORAGE_KEY)
  }, [])

  return { walletState: localStorageState, aptosAccount, updateWalletState, signOut }
}

export const [WalletStateProvider, useWalletStateContext] = constate(useWalletState)
