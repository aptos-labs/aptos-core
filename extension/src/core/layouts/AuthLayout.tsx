// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { Box } from '@chakra-ui/react';
import useWalletState from 'core/hooks/useWalletState';
import React, { useMemo } from 'react';
import { Navigate } from 'react-router-dom';
import { RoutePaths } from 'core/routes';

interface AuthLayoutProps {
  children: React.ReactNode,
  routePath: RoutePaths;
}

export default function AuthLayout({
  children,
  routePath,
}: AuthLayoutProps) {
  const { aptosAccount } = useWalletState();
  const page = <Box width="100%" height="100%">{children}</Box>;

  const redirectPath = useMemo(() => {
    switch (routePath) {
      case '/':
      case '/create-wallet':
      case '/help':
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
  }, []);

  const redirect = <Navigate to={redirectPath} />;

  switch (routePath) {
    case '/':
    case '/create-wallet':
    case '/help':
      return aptosAccount ? redirect : page;
    case '/gallery':
    case '/password':
    case '/settings':
    case '/settings/credentials':
    case '/settings/network':
    case '/tokens/:id':
    case '/wallet':
      return aptosAccount ? page : redirect;
    default:
      return page;
  }
}
