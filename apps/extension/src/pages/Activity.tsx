// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { Spinner, VStack } from '@chakra-ui/react';
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
      <VStack width="100%" p={4}>
        {
          activity.isLoading || activity.isFetchingNextPage
            ? <Spinner />
            : <TransactionList transactions={transactions} />
        }
        <NextPageLoader query={activity} />
      </VStack>
    </WalletLayout>
  );
}

export default Activity;
