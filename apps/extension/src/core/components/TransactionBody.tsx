// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import React from 'react';
import {
  Box,
  Button,
  Heading,
  HStack,
  Text,
  Tooltip,
  VStack,
} from '@chakra-ui/react';
import { FaRegCheckCircle } from '@react-icons/all-files/fa/FaRegCheckCircle';
import { FaRegTimesCircle } from '@react-icons/all-files/fa/FaRegTimesCircle';
import { ScriptFunctionPayload } from 'aptos/dist/api/data-contracts';
import { ExternalLinkIcon } from '@chakra-ui/icons';
import { collapseHexString } from 'core/utils/hex';
import useWalletState from 'core/hooks/useWalletState';
import { MaybeHexString } from 'aptos';
import { useParams } from 'react-router-dom';
import { useUserTransaction } from 'core/queries/transaction';
import Copyable from './Copyable';

interface DetailItemProps {
  children: React.ReactNode,
  label: string,
}

function DetailItem({ children, label }: DetailItemProps) {
  return (
    <HStack w="100%" justify="space-between">
      <Text as="span" fontWeight={700}>{ label }</Text>
      { children }
    </HStack>
  );
}

function copyableHexString(hexString: MaybeHexString) {
  return <Copyable value={hexString}>{ collapseHexString(hexString, 12) }</Copyable>;
}

function useTransactionDetails(version: string) {
  const { data: txn } = useUserTransaction({ txnHashOrVersion: version });

  if (!txn) {
    return null;
  }

  const datetime = new Date(Number(txn.timestamp) / 1000);
  const fullDatetime = datetime.toLocaleDateString('en-us', {
    day: 'numeric',
    hour: 'numeric',
    minute: 'numeric',
    month: 'short',
    year: 'numeric',
  });

  const payload = txn.payload as ScriptFunctionPayload;
  const recipient = payload.arguments[0] as string;
  const amount = Number(payload.arguments[1]);
  const coinName = payload.type_arguments[0].split('::').pop();

  return {
    amount,
    coinName,
    fullDatetime,
    recipient,
    ...txn,
  };
}

function TransactionBody() {
  const { aptosAccount } = useWalletState();
  const { version } = useParams();

  const txn = useTransactionDetails(version!);
  const userAddress = aptosAccount!.address().hex();

  return txn && (
    <VStack
      w="100%"
      pt={4}
      px={4}
      alignItems="stretch"
    >
      <Heading mb={4}>
        Transaction detail
      </Heading>
      <Button
        fontSize="md"
        fontWeight={400}
        as="a"
        target="_blank"
        rightIcon={<ExternalLinkIcon />}
        variant="unstyled"
        cursor="pointer"
        href={`https://explorer.devnet.aptos.dev/txn/${txn.version}`}
      >
        View on Aptos explorer
      </Button>
      <DetailItem label="From">
        { txn.sender === userAddress ? <Text>You</Text> : copyableHexString(txn.sender) }
      </DetailItem>
      <DetailItem label="To">
        { txn.recipient === userAddress ? <Text>You</Text> : copyableHexString(txn.recipient) }
      </DetailItem>
      <DetailItem label="Amount">
        <Text>{ `${txn.amount} ${txn.coinName}` }</Text>
      </DetailItem>
      <DetailItem label="Version">
        <Text>{ txn.version }</Text>
      </DetailItem>
      <DetailItem label="Hash">
        <Copyable prompt="Copy full hash" value={txn.hash}>
          { collapseHexString(txn.hash) }
        </Copyable>
      </DetailItem>
      <DetailItem label="Timestamp">
        <Copyable prompt="Copy timestamp" value={txn.timestamp}>
          { txn.fullDatetime }
        </Copyable>
      </DetailItem>
      <DetailItem label="Status">
        <Tooltip label={txn.vm_status}>
          <Box color={txn.success ? 'green.400' : 'red.400'}>
            { txn.success ? <FaRegCheckCircle /> : <FaRegTimesCircle /> }
          </Box>
        </Tooltip>
      </DetailItem>
      <DetailItem label="Gas used">
        <Text>{ txn.gas_used }</Text>
      </DetailItem>
    </VStack>
  );
}

export default TransactionBody;
