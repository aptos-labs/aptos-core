// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import React, { useMemo } from 'react';
import {
  Box,
  Button,
  Center,
  Spinner,
  Text,
  useColorMode,
  VStack,
} from '@chakra-ui/react';
import { Types } from 'aptos';
import { secondaryBorderColor } from 'core/colors';
import ActivityItem from 'core/components/ActivityItem';
import { useActiveAccount } from 'core/hooks/useAccounts';
import { useCoinTransferTransactions } from 'core/queries/transaction';
import { ChevronRightIcon } from '@chakra-ui/icons';
import ChakraLink from './ChakraLink';
import { Routes } from '../routes';

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
  limit?: number;
  transactions?: Types.UserTransaction[]
}

export function TransactionList({
  isLoading,
  limit,
  transactions,
}: TransactionListProps) {
  const { activeAccountAddress } = useActiveAccount();
  const {
    data: hookTransactions,
    isLoading: hookIsLoading,
  } = useCoinTransferTransactions(activeAccountAddress, { enabled: !transactions });

  const masterIsLoading = (isLoading) || hookIsLoading;
  const sortedTxns = (transactions) || hookTransactions?.sort(
    (a, b) => Number(b.version) - Number(a.version),
  );

  const children = useMemo(
    () => {
      if (!(sortedTxns && sortedTxns.length > 0)) {
        return <NoActivity />;
      }
      let result = sortedTxns.map((t) => <ActivityItem key={t.hash} transaction={t} />);
      const prevResultLength = result.length;

      if (limit) {
        result = result.slice(0, limit);
        if (limit < prevResultLength) {
          result.push((
            <ChakraLink key="View more" width="100%" to={Routes.activity.path}>
              <Button
                py={6}
                width="100%"
                rightIcon={<ChevronRightIcon />}
                justifyContent="space-between"
              >
                View more
              </Button>
            </ChakraLink>
          ));
        }
      }
      return result;
    },
    [limit, sortedTxns],
  );

  return (
    <VStack w="100%" spacing={3}>
      { masterIsLoading ? <Spinner /> : children }
    </VStack>
  );
}

export default TransactionList;
