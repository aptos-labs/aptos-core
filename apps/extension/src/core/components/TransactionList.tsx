// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import React, { useMemo } from 'react';
import {
  Box,
  Center,
  Spinner,
  Text,
  useColorMode,
  VStack,
} from '@chakra-ui/react';
import { Types } from 'aptos';
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
  isLoading?: boolean,
  transactions?: Types.UserTransaction[]
}

export function TransactionList({
  isLoading,
  transactions,
}: TransactionListProps) {
  const children = useMemo(
    () => ((transactions && transactions.length > 0)
      ? transactions.map((t) => <ActivityItem key={t.hash} transaction={t} />)
      : <NoActivity />),
    [transactions],
  );

  return (
    <VStack w="100%" spacing={3}>
      { isLoading ? <Spinner /> : children }
    </VStack>
  );
}

export default TransactionList;
