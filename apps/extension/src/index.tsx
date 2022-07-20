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
import { createStandaloneToast } from '@chakra-ui/toast';
import SimulatedExtensionContainer from 'core/layouts/SimulatedExtensionContainer';
import { StepsStyleConfig as Steps } from 'chakra-ui-steps';

const { ToastContainer } = createStandaloneToast();

const isProductionEnv = process.env.NODE_ENV === 'production';

// todo: fix for extension
// ReactGA.initialize('G-VFLV1PF59M');
// ReactGA.send({
//   hitType: 'pageview',
//   network: getLocalStorageNetworkState(),
//   page: window.location.pathname + window.location.search,
// });

const theme: ThemeConfig = extendTheme({
  components: {
    Steps,
  },
  initialColorMode: 'light',
  styles: {
    global: {
      'html, body': {
        margin: 0,
        overflow: isProductionEnv ? 'hidden' : undefined,
        padding: 0,
      },
    },
  },
  useSystemColorMode: false,
});

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      refetchOnWindowFocus: isProductionEnv,
    },
  },
});

const root = createRoot(document.getElementById('root') as Element);

root.render(
  <StrictMode>
    <QueryClientProvider client={queryClient}>
      <ChakraProvider theme={theme}>
        <WalletStateProvider>
          <SimulatedExtensionContainer>
            <MemoryRouter>
              <Routes>
                {
                  Object.values(PageRoutes).map(({ element, routePath }) => (
                    <Route key={routePath} path={routePath} element={element} />
                  ))
                }
              </Routes>
            </MemoryRouter>
          </SimulatedExtensionContainer>
        </WalletStateProvider>
      </ChakraProvider>
    </QueryClientProvider>
    <ToastContainer />
  </StrictMode>,
);
