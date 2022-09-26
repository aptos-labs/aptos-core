// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  Box,
  Divider,
  Heading,
  Spinner,
  VStack,
} from '@chakra-ui/react';
import React, { useMemo } from 'react';
import { useParams } from 'react-router-dom';
import { Types } from 'aptos';
import GraceHopperBoringAvatar from 'core/components/BoringAvatar';
import Copyable from 'core/components/Copyable';
import NextPageLoader from 'core/components/NextPageLoader';
import TransactionList from 'core/components/TransactionList';
import useActivity from 'core/queries/useActivity';
import WalletLayout from 'core/layouts/WalletLayout';
import { collapseHexString } from 'core/utils/hex';

type UserTransaction = Types.UserTransaction;
type EntryFunctionPayload = Types.EntryFunctionPayload;

function txnParties(txn: UserTransaction) {
  const payload = txn.payload as EntryFunctionPayload;
  const recipient = payload.arguments[0];
  return [txn.sender, recipient];
}

function Account() {
  const { address } = useParams();
  const activity = useActivity();

  const transactions = useMemo(
    () => activity.data?.pages
      .flatMap((page) => page.txns)
      .filter((txn) => txnParties(txn).includes(address)),
    [activity.data, address],
  );

  return (
    <WalletLayout title="Account" showBackButton>
      <VStack width="100%" paddingTop={8} px={4} spacing={4}>
        <Box w={20}>
          <GraceHopperBoringAvatar type="beam" />
        </Box>
        <Heading fontSize="lg" fontWeight={500} mb={8}>
          <Copyable value={address!}>
            { collapseHexString(address!, 12) }
          </Copyable>
        </Heading>
        <Divider />
        <Heading fontSize="lg">Between you</Heading>
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

export default Account;
