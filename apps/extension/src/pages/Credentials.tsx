// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import React from 'react';
import {
  Box,
  VStack,
} from '@chakra-ui/react';
import WalletLayout from 'core/layouts/WalletLayout';
import CredentialsBody from 'core/components/CredentialsBody';

function Credentials() {
  return (
    <WalletLayout title="Credentials" showBackButton>
      <VStack width="100%" paddingTop={8}>
        <Box px={4} pb={4} width="100%">
          <CredentialsBody />
        </Box>
      </VStack>
    </WalletLayout>
  );
}

export default Credentials;
