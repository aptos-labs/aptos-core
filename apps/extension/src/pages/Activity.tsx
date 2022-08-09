// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  Heading,
  VStack,
} from '@chakra-ui/react';
import React from 'react';
import WalletLayout from 'core/layouts/WalletLayout';
import AuthLayout from 'core/layouts/AuthLayout';
import { Routes as PageRoutes } from 'core/routes';
import { useCoinTransferTransactions } from 'core/queries/transaction';
import { useWalletState } from 'core/hooks/useWalletState';
import TransactionList from 'core/components/TransactionList';

function Activity() {
  const { aptosAccount } = useWalletState();
  const { data: transactions, isFetching } = useCoinTransferTransactions({
    address: aptosAccount?.address().hex(),
  });

  const sortedTxns = !isFetching
    ? transactions?.sort((a, b) => Number(b.version) - Number(a.version))
    : undefined;

  return (
    <AuthLayout routePath={PageRoutes.activity.routePath}>
      <WalletLayout>
        <VStack width="100%" paddingTop={8} px={4} alignItems="start">
          <Heading fontSize="xl" mb={4}>Activity</Heading>
          <TransactionList transactions={sortedTxns} />
        </VStack>
      </WalletLayout>
    </AuthLayout>
  );
}

export default Activity;
