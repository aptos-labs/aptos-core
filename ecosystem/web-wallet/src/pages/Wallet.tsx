// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import React from 'react'
import { useGlobalState } from '../GlobalState'

import './App.css'

export default function Wallet () {
  const [state] = useGlobalState()
  const address = state.account?.address().hex()
  const pubKey = state.account?.pubKey().hex()
  return (
    <div className="App-header">
      <h2>Aptos Wallet </h2>
      <p>Address: {address}</p>
      <p>Public Key: {pubKey}</p>
    </div>
  )
}
