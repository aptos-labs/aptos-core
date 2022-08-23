// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import React from 'react';
import {
  Box,
  VStack,
} from '@chakra-ui/react';
import WalletLayout from 'core/layouts/WalletLayout';
import NetworkBody from 'core/components/NetworkBody';
import AuthLayout from 'core/layouts/AuthLayout';
import { Routes as PageRoutes } from 'core/routes';

function Network() {
  return (
    <AuthLayout routePath={PageRoutes.network.path}>
      <WalletLayout title="Network" showBackButton>
        <VStack width="100%" paddingTop={8}>
          <Box px={4} pb={4} width="100%">
            <NetworkBody />
          </Box>
        </VStack>
      </WalletLayout>
    </AuthLayout>
  );
}

export default Network;
