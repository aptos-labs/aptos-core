// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { Box } from '@chakra-ui/react';
import { useWalletState } from 'core/hooks/useWalletState';
import React, { useMemo } from 'react';
import { Navigate } from 'react-router-dom';
import { RoutePaths } from 'core/routes';
import { isWalletLocked } from 'core/utils/account';
import { WALLET_STATE_LOADED_KEY } from 'core/constants';

interface AuthLayoutProps {
  children: React.ReactNode,
  routePath: RoutePaths;
}

export default function AuthLayout({
  children,
  routePath,
}: AuthLayoutProps) {
  const { accounts, aptosAccount } = useWalletState();
  const page = <Box width="100%" height="100%">{children}</Box>;
  console.log(accounts);

  const redirectPath = useMemo(() => {
    switch (routePath) {
      case '/':
      case '/create-wallet':
      case '/help':
      case '/no-wallet':
      case '/add-account':
      case '/create-account':
      case '/import/private-key':
      case '/import/mnemonic':
        return '/wallet';
      case '/gallery':
      case '/password':
      case '/settings':
      case '/settings/credentials':
      case '/settings/network':
      case '/tokens/:id':
      case '/wallet':
        return '/';
      default:
        return '/';
    }
  }, [routePath]);

  // If needed, we should load the accounts into sessionStorage
  if (!sessionStorage.getItem(WALLET_STATE_LOADED_KEY)) {
    return <Navigate to="/load-state" />;
  } if (isWalletLocked()) {
    return <Navigate to="/password" />;
  }
  const redirect = <Navigate to={redirectPath} />;

  switch (routePath) {
    case '/':
    case '/create-wallet':
    case '/help':
    case '/no-wallet':
      return aptosAccount ? redirect : page;
    case '/gallery':
    case '/password':
    case '/settings':
    case '/settings/credentials':
    case '/settings/network':
    case '/tokens/:id':
    case '/wallet':
      return aptosAccount ? page : redirect;
    case '/add-account':
    case '/create-account':
    case '/import/private-key':
    case '/import/mnemonic':
      return page;
    default:
      return page;
  }
}
