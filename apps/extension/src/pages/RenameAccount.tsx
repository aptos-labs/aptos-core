// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { Box, VStack } from '@chakra-ui/react';
import React from 'react';
import WalletLayout from 'core/layouts/WalletLayout';
import RenameAccountBody from 'core/components/RenameAccountBody';

function RenameAccount() {
  return (
    <WalletLayout
      title="Change Wallet Name"
      showBackButton
      showAccountCircle={false}
      hasWalletFooter={false}
    >
      <VStack width="100%" paddingTop={8} height="100%">
        <Box width="100%" height="100%">
          <RenameAccountBody />
        </Box>
      </VStack>
    </WalletLayout>
  );
}

export default RenameAccount;
