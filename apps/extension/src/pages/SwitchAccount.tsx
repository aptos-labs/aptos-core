// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import React from 'react';
import {
  Box,
} from '@chakra-ui/react';
import WalletLayout from 'core/layouts/WalletLayout';
import SwitchAccountBody from 'core/components/SwitchAccountBody';

function SwitchAccount() {
  return (
    <WalletLayout title="Accounts" showBackButton showAccountCircle={false} hasWalletFooter={false}>
      <Box width="100%" height="100%">
        <SwitchAccountBody />
      </Box>
    </WalletLayout>
  );
}

export default SwitchAccount;
