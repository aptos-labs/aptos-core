// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  extendTheme,
  ChakraProvider,
} from '@chakra-ui/react';
import React from 'react';
import { createRoot } from 'react-dom/client';
import { MemoryRouter } from 'react-router-dom';
import { QueryClient, QueryClientProvider } from 'react-query';
import { AppStateProvider } from 'core/hooks/useAppState';
import Prompt from './Prompt';
import { PromptStateProvider } from './hooks';

const isProductionEnv = process.env.NODE_ENV === 'production';

const theme = extendTheme({
  initialColorMode: 'light',
  styles: {
    global: {
      'html, body': {
        margin: 0,
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

const root = createRoot(document.getElementById('prompt') as Element);
root.render(
  <ChakraProvider theme={theme}>
    <QueryClientProvider client={queryClient}>
      <AppStateProvider>
        <PromptStateProvider>
          <MemoryRouter>
            <Prompt />
          </MemoryRouter>
        </PromptStateProvider>
      </AppStateProvider>
    </QueryClientProvider>
  </ChakraProvider>,
);
