// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import React from 'react';
import {
  Box,
  Center,
  Spinner,
  Text,
  useColorMode,
  VStack,
} from '@chakra-ui/react';
import { UserTransaction } from 'aptos/src/api/data-contracts';
import { secondaryBorderColor } from 'core/colors';
import ActivityItem from 'core/components/ActivityItem';

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
  transactions?: UserTransaction[],
}

export function TransactionList({ transactions }: TransactionListProps) {
  return (
    <VStack w="100%" spacing={3}>
      { !transactions && <Spinner /> }
      { transactions && transactions.length > 0
        ? transactions.map((t) => <ActivityItem key={t.hash} transaction={t} />)
        : <NoActivity /> }
    </VStack>
  );
}

export default TransactionList;
