// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  VStack,
} from '@chakra-ui/react';
import React from 'react';
import WalletLayout from 'core/layouts/WalletLayout';
import TransactionList from 'core/components/TransactionList';

function Activity() {
  return (
    <WalletLayout title="Activity">
      <VStack width="100%" paddingTop={8} px={4} alignItems="start">
        <TransactionList />
      </VStack>
    </WalletLayout>
  );
}

export default Activity;
