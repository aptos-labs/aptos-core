// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import React, { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { MemoryRouter, Routes, Route } from 'react-router-dom'
import { ChakraProvider, extendTheme, type ThemeConfig } from '@chakra-ui/react'
import Wallet from './pages/Wallet'
import Login from './pages/Login'
import { WalletStateProvider } from './hooks/useWalletState'
import Help from './pages/Help'

const theme: ThemeConfig = extendTheme({
  initialColorMode: 'dark',
  useSystemColorMode: false,
  styles: {
    global: {
      'html, body': {
        margin: 0,
        padding: 0
      }
    }
  }
})

const root = createRoot(document.getElementById('root') as Element)

root.render(
  <StrictMode>
    <ChakraProvider theme={theme}>
      <WalletStateProvider>
        <MemoryRouter>
          <Routes>
            <Route path='/' element={<Login/>}></Route>
            <Route path='/wallet' element={<Wallet/>}></Route>
            <Route path="/help" element={<Help />}></Route>
          </Routes>
        </MemoryRouter>
      </WalletStateProvider>
    </ChakraProvider>
  </StrictMode>
)
