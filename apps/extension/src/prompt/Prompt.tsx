// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import React from 'react';
import { useRoutes } from 'react-router-dom';
import { AccountsProvider } from 'core/hooks/useAccounts';
import { useAppState } from 'core/hooks/useAppState';
import { NetworksProvider } from 'core/hooks/useNetworks';
import { PermissionRequestContextProvider, usePromptState } from './hooks';
import { routes } from './routes';

export default function Prompt() {
  const promptRoutes = useRoutes(routes);
  const { isAppStateReady } = useAppState();
  const { permissionRequest } = usePromptState();

  // Pause rendering until state is ready
  return isAppStateReady && permissionRequest !== undefined ? (
    <AccountsProvider>
      <NetworksProvider>
        <PermissionRequestContextProvider permissionRequest={permissionRequest}>
          { promptRoutes }
        </PermissionRequestContextProvider>
      </NetworksProvider>
    </AccountsProvider>
  ) : null;
}
