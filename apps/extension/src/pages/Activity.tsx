// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  Box,
  Center,
  Heading,
  Text,
  useColorMode,
  VStack,
} from '@chakra-ui/react';
import React from 'react';
import withSimulatedExtensionContainer from 'core/components/WithSimulatedExtensionContainer';
import WalletLayout from 'core/layouts/WalletLayout';
import AuthLayout from 'core/layouts/AuthLayout';
import { secondaryBorderColor } from 'core/colors';
import { Routes as PageRoutes } from 'core/routes';
import { ActivityItem } from 'core/components/ActivityItem';
import { useCoinTransferTransactions } from 'core/queries/transaction';

function NoActivity() {
  const { colorMode } = useColorMode();
  return (
    <Box w="100%" borderWidth="1px" borderRadius=".5rem" borderColor={secondaryBorderColor[colorMode]}>
      <Center height="100%" p={4}>
        <Text fontSize="md" textAlign="center">No activity yet!</Text>
      </Center>
    </Box>
  );
}

function Activity() {
  const { data: transactions } = useCoinTransferTransactions();
  const sortedTxns = transactions?.sort((a, b) => Number(b.version) - Number(a.version));

  return (
    <AuthLayout routePath={PageRoutes.activity.routePath}>
      <WalletLayout>
        <VStack width="100%" paddingTop={8} px={4} alignItems="start">
          <Heading fontSize="xl" mb={4}>Activity</Heading>
          <VStack w="100%" spacing={3}>
            {
              (sortedTxns && sortedTxns.length > 0)
                ? sortedTxns.map((t) => <ActivityItem key={t.hash} isSent transaction={t} />)
                : <NoActivity />
            }
          </VStack>
        </VStack>
      </WalletLayout>
    </AuthLayout>
  );
}

export default withSimulatedExtensionContainer({ Component: Activity });
