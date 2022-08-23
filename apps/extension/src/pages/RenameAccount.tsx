// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { Box, VStack } from '@chakra-ui/react';
import React from 'react';
import AuthLayout from 'core/layouts/AuthLayout';
import { Routes as PageRoutes } from 'core/routes';
import WalletLayout from 'core/layouts/WalletLayout';
import RenameAccountBody from 'core/components/RenameAccountBody';

function RenameAccount() {
  return (
    <AuthLayout routePath={PageRoutes.rename_account.path}>
      <WalletLayout title="Rename Account" showBackButton>
        <VStack width="100%" paddingTop={8}>
          <Box px={4} pb={4} width="100%">
            <RenameAccountBody />
          </Box>
        </VStack>
      </WalletLayout>
    </AuthLayout>
  );
}

export default RenameAccount;
