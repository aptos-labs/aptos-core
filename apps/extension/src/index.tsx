// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import React, { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import {
  QueryClientProvider,
  QueryClient,
} from 'react-query';
import { Route, MemoryRouter, Routes } from 'react-router-dom';
import { ChakraProvider, extendTheme, type ThemeConfig } from '@chakra-ui/react';
import { Routes as PageRoutes } from 'core/routes';
import { WalletStateProvider } from 'core/hooks/useWalletState';
import ReactGA from 'react-ga4';
import { getLocalStorageNetworkState } from 'core/utils/network';

ReactGA.initialize('G-VFLV1PF59M');
ReactGA.send({
  hitType: 'pageview',
  network: getLocalStorageNetworkState(),
  page: window.location.pathname + window.location.search,
});

const theme: ThemeConfig = extendTheme({
  initialColorMode: 'light',
  styles: {
    global: {
      'html, body': {
        margin: 0,
        overflow: (process.env.NODE_ENV !== 'development') ? 'hidden' : undefined,
        padding: 0,
      },
    },
  },
  useSystemColorMode: false,
});

const queryClient = new QueryClient();

const root = createRoot(document.getElementById('root') as Element);

root.render(
  <StrictMode>
    <QueryClientProvider client={queryClient}>
      <ChakraProvider theme={theme}>
        <WalletStateProvider>
          <MemoryRouter>
            <Routes>
              {
                Object.values(PageRoutes).map(({ element, routePath }) => (
                  <Route key={routePath} path={routePath} element={element} />
                ))
              }
            </Routes>
          </MemoryRouter>
        </WalletStateProvider>
      </ChakraProvider>
    </QueryClientProvider>
  </StrictMode>,
);
