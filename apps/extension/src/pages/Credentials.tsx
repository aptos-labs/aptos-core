// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import React from 'react';
import {
  Box,
  VStack,
} from '@chakra-ui/react';
import WalletLayout from 'core/layouts/WalletLayout';
import CredentialsBody from 'core/components/CredentialsBody';
import AuthLayout from 'core/layouts/AuthLayout';
import { Routes as PageRoutes } from 'core/routes';

function Credentials() {
  return (
    <AuthLayout routePath={PageRoutes.credentials.path}>
      <WalletLayout title="Credentials" showBackButton>
        <VStack width="100%" paddingTop={8}>
          <Box px={4} pb={4} width="100%">
            <CredentialsBody />
          </Box>
        </VStack>
      </WalletLayout>
    </AuthLayout>

  );
}

export default Credentials;
