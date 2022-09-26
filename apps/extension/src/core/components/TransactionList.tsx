// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import React from 'react';
import {
  Box,
  Center,
  Text,
  useColorMode,
  VStack,
} from '@chakra-ui/react';
import { Types } from 'aptos';
import { secondaryBorderColor } from 'core/colors';
import TransactionListItem from 'core/components/TransactionListItem';

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

interface TransactionListProps {
  transactions: Types.UserTransaction[] | undefined
}

export function TransactionList({
  transactions,
}: TransactionListProps) {
  const hasTransactions = transactions !== undefined && transactions.length > 0;
  return (
    <VStack w="100%" spacing={3}>
      {
        hasTransactions
          ? transactions.map((t) => <TransactionListItem key={t.version} transaction={t} />)
          : <NoActivity />
      }
    </VStack>
  );
}

export default TransactionList;
