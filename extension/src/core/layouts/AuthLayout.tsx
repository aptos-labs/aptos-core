// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { Box } from '@chakra-ui/react';
import useWalletState from 'core/hooks/useWalletState';
import React from 'react';
import { Navigate } from 'react-router-dom';

interface AuthLayoutProps {
  children: React.ReactNode,
  redirectPath: string;
}

export default function AuthLayout({
  children,
  redirectPath,
}: AuthLayoutProps) {
  const { aptosAccount } = useWalletState();
  return aptosAccount
    ? <Box width="100%" height="100%">{children}</Box>
    : <Navigate to={redirectPath} />;
}
