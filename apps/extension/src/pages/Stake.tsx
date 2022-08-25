// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  VStack,
} from '@chakra-ui/react';
import React from 'react';
import WalletLayout from 'core/layouts/WalletLayout';
import StakeBody from 'core/components/StakeBody';

function Stake() {
  return (
    <WalletLayout title="Stake">
      <VStack width="100%" paddingTop={8} px={4}>
        <StakeBody />
      </VStack>
    </WalletLayout>
  );
}

export default Stake;
