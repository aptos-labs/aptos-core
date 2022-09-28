// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  Box,
  Center,
  Circle,
  Heading,
  Spinner,
  Text,
  VStack,
} from '@chakra-ui/react';
import React, { useMemo } from 'react';
import WalletLayout from 'core/layouts/WalletLayout';
import ActivityList from 'core/components/ActivityList';
import NextPageLoader from 'core/components/NextPageLoader';
import useActivity from 'core/queries/useActivity';
import { IoReceiptOutline } from '@react-icons/all-files/io5/IoReceiptOutline';

function NoActivity() {
  return (
    <VStack w="62%" spacing={2}>
      <Circle size="57px" color="navy.500" bgColor="#b3b3b31a" mb={4}>
        <IoReceiptOutline size="26px" />
      </Circle>
      <Heading fontSize="xl" color="navy.900">No activity yet</Heading>
      <Text fontSize="md" color="navy.600" textAlign="center">
        All of your transactions and dApp interactions will show up here.
      </Text>
    </VStack>
  );
}

function Activity() {
  const activity = useActivity();

  const transactions = useMemo(
    () => activity.data?.pages.flatMap((page) => page.txns),
    [activity.data],
  );

  return (
    <WalletLayout title="Activity">
      {
        activity.isLoading || activity.isFetchingNextPage
          ? (
            <Center h="100%">
              <Spinner size="xl" thickness="4px" />
            </Center>
          )
          : null
      }
      {
        transactions && transactions.length === 0
          ? <Center h="100%"><NoActivity /></Center>
          : null
      }
      {
        transactions && transactions.length > 0
          ? (
            <Box pt={3}>
              <ActivityList transactions={transactions} />
              <NextPageLoader query={activity} />
            </Box>
          )
          : null
      }
    </WalletLayout>
  );
}

export default Activity;
