// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import React from 'react'
import { AptosAccount } from 'aptos'

export type GlobalState = {
  account?: AptosAccount,
}

const defaultGlobalState: GlobalState = {
  account: undefined
}

function reducer (state: GlobalState, newValue: GlobalState): GlobalState {
  return { ...state, ...newValue }
}

export const GlobalStateContext = React.createContext(defaultGlobalState)
export const DispatchStateContext = React.createContext<React.Dispatch<GlobalState>>((value: GlobalState) => value)

export const GlobalStateProvider = ({ children }: { children: React.ReactNode }) => {
  const [state, dispatch] = React.useReducer(reducer, defaultGlobalState)
  return (
    <GlobalStateContext.Provider value={state}>
      <DispatchStateContext.Provider value={dispatch}>
        {children}
      </DispatchStateContext.Provider>
    </GlobalStateContext.Provider>
  )
}

export const useGlobalState = (): [GlobalState, React.Dispatch<GlobalState>] => {
  return [
    React.useContext(GlobalStateContext),
    React.useContext(DispatchStateContext)
  ]
}
