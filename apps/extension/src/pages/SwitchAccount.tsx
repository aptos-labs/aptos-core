// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import React from 'react';
import {
  Box,
  VStack,
} from '@chakra-ui/react';
import WalletLayout from 'core/layouts/WalletLayout';
import SwitchAccountBody from 'core/components/SwitchAccountBody';

function SwitchAccount() {
  return (
    <WalletLayout title="Accounts" showAccountCircle={false}>
      <VStack width="100%" paddingTop={8} height="100%">
        <Box px={4} pb={4} width="100%" height="100%">
          <SwitchAccountBody />
        </Box>
      </VStack>
    </WalletLayout>
  );
}

export default SwitchAccount;
