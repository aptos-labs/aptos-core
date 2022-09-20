// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { VStack } from '@chakra-ui/react';
import React, { useMemo } from 'react';
import WalletLayout from 'core/layouts/WalletLayout';
import NextPageLoader from 'core/components/NextPageLoader';
import TransactionList from 'core/components/TransactionList';
import useActivity from 'core/queries/useActivity';

function Activity() {
  const activity = useActivity();

  const transactions = useMemo(
    () => activity.data?.pages.flatMap((page) => page.txns),
    [activity.data],
  );

  return (
    <WalletLayout title="Activity">
      <VStack width="100%" p={4} alignItems="start">
        <TransactionList
          isLoading={activity.isLoading || activity.isFetchingNextPage}
          transactions={transactions}
        />
        <NextPageLoader query={activity} />
      </VStack>
    </WalletLayout>
  );
}

export default Activity;
