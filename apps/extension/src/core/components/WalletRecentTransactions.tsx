// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  Button,
  Heading,
  Spinner,
  VStack,
  useColorMode,
} from '@chakra-ui/react';
import React, { useMemo } from 'react';
import { secondaryTextColor } from 'core/colors';
import ChakraLink from 'core/components/ChakraLink';
import useActivity from 'core/queries/useActivity';
import { ActivityList } from './ActivityList';
import { Routes } from '../routes';

export default function WalletRecentTransactions() {
  const { colorMode } = useColorMode();

  const activity = useActivity({ pageSize: 5 });
  const transactions = useMemo(() => activity.data?.pages[0]?.txns, [activity.data]);

  const hasActivity = transactions !== undefined && transactions.length > 0;
  if (!hasActivity) {
    return null;
  }

  return (
    <VStack spacing={2} alignItems="stretch">
      <Heading
        px={4}
        py={2}
        fontSize="md"
        color={secondaryTextColor[colorMode]}
      >
        RECENT TRANSACTIONS
      </Heading>
      {
        activity.isLoading
          ? <Spinner />
          : <ActivityList transactions={transactions} />
      }
      {
        activity.hasNextPage
          ? (
            <ChakraLink to={Routes.activity.path}>
              <Button w="100%" variant="unstyled" color="green.500">
                View all transactions
              </Button>
            </ChakraLink>
          )
          : null
      }
    </VStack>
  );
}
