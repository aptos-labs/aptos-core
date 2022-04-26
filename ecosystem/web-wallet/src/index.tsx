// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import React from 'react'
import ReactDOM from 'react-dom'
import { MemoryRouter, Routes, Route } from 'react-router-dom'
import Wallet from './pages/Wallet'
import Login from './pages/Login'
import { GlobalStateProvider } from './GlobalState'

ReactDOM.render(
  <GlobalStateProvider>
    <MemoryRouter>
      <Routes>
        <Route path='/' element={<Login/>}></Route>
        <Route path='/wallet' element={<Wallet/>}></Route>
      </Routes>
    </MemoryRouter>
  </GlobalStateProvider>,
  document.getElementById('root')
)
