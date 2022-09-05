// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  VStack,
} from '@chakra-ui/react';
import React from 'react';
import WalletLayout from 'core/layouts/WalletLayout';
import { useCoinTransferTransactions } from 'core/queries/transaction';
import TransactionList from 'core/components/TransactionList';
import { useActiveAccount } from 'core/hooks/useAccounts';

function Activity() {
  const { activeAccountAddress } = useActiveAccount();
  const {
    data: transactions,
    isLoading,
  } = useCoinTransferTransactions(activeAccountAddress);
  const sortedTxns = transactions?.sort((a, b) => Number(b.version) - Number(a.version));

  return (
    <WalletLayout title="Activity">
      <VStack width="100%" paddingTop={8} px={4} alignItems="start">
        <TransactionList transactions={sortedTxns} isLoading={isLoading} />
      </VStack>
    </WalletLayout>
  );
}

export default Activity;
