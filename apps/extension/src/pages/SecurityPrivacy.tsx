// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import React from 'react';
import {
  Box,
  VStack,
} from '@chakra-ui/react';
import WalletLayout from 'core/layouts/WalletLayout';

function SecurityPrivacy() {
  // TODO: will implement later
  return (
    <WalletLayout title="Security and Privacy" showBackButton>
      <VStack width="100%" paddingTop={8}>
        <Box px={4} pb={4} width="100%" />
      </VStack>
    </WalletLayout>
  );
}

export default SecurityPrivacy;
