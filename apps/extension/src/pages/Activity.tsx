// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  Heading,
  VStack,
} from '@chakra-ui/react';
import React from 'react';
import withSimulatedExtensionContainer from 'core/components/WithSimulatedExtensionContainer';
import WalletLayout from 'core/layouts/WalletLayout';
import AuthLayout from 'core/layouts/AuthLayout';
import { Routes as PageRoutes } from 'core/routes';
import { ActivityItem } from 'core/components/ActivityItem';
import { useCoinTransferTransactions } from 'core/queries/transaction';

function Activity() {
  const { data: transactions } = useCoinTransferTransactions();

  const sortedTransactions = transactions
    ?.sort((a, b) => Number(b.version) - Number(a.version));

  return (
    <AuthLayout routePath={PageRoutes.activity.routePath}>
      <WalletLayout>
        <VStack width="100%" paddingTop={8} px={4} alignItems="start">
          <Heading fontSize="xl" mb={4}>Activity</Heading>
          <VStack w="100%" spacing={3}>
            { sortedTransactions?.map((t) => <ActivityItem key={t.hash} isSent transaction={t} />) }
          </VStack>
        </VStack>
      </WalletLayout>
    </AuthLayout>
  );
}

export default withSimulatedExtensionContainer({ Component: Activity });
