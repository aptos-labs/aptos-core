// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { Box } from '@chakra-ui/react';
import React from 'react';
import { RoutePaths } from 'core/routes';

interface AuthLayoutProps {
  children: React.ReactNode,
  routePath: RoutePaths;
}

export default function AuthLayout({
  children,
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  routePath,
}: AuthLayoutProps) {
  return <Box width="100%" height="100%">{children}</Box>;
}
