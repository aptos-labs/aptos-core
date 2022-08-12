// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import React from 'react';
import {
  Badge,
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
import { ScriptFunctionPayload } from 'aptos/dist/generated';
import { ExternalLinkIcon } from '@chakra-ui/icons';
import { collapseHexString } from 'core/utils/hex';
import { HexString, MaybeHexString } from 'aptos';
import { useParams } from 'react-router-dom';
import { useTransaction } from 'core/queries/transaction';
import ChakraLink from 'core/components/ChakraLink';
import useGlobalStateContext from 'core/hooks/useGlobalState';
import Copyable from './Copyable';

interface DetailItemProps {
  children: React.ReactNode,
  label: string,
}

function DetailItem({ children, label }: DetailItemProps) {
  return (
    <HStack w="100%" fontSize="md" justify="space-between">
      <Text as="span" fontWeight={700}>{ label }</Text>
      { children }
    </HStack>
  );
}

function useTransactionDetails(version?: bigint) {
  const { data: txn } = useTransaction(version);
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
  const { activeAccountAddress } = useGlobalStateContext();
  const { version } = useParams();

  const txn = useTransactionDetails(version ? BigInt(version) : undefined);
  const userAddress = activeAccountAddress
    && HexString.ensure(activeAccountAddress).toShortString();

  function clickableAddress(address: MaybeHexString) {
    return address === userAddress
      ? <Text>You</Text>
      : (
        <Badge fontSize="sm" textTransform="none">
          <ChakraLink to={`/accounts/${address}`}>
            { collapseHexString(address, 12) }
          </ChakraLink>
        </Badge>
      );
  }

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
        { clickableAddress(txn.sender) }
      </DetailItem>
      <DetailItem label="To">
        { clickableAddress(txn.recipient) }
      </DetailItem>
      <DetailItem label="Amount">
        <Text>{ `${txn.amount} ${txn.coinName}` }</Text>
      </DetailItem>
      <DetailItem label="Version">
        <Text>{ txn.version }</Text>
      </DetailItem>
      <DetailItem label="Hash">
        <Copyable prompt="Copy full hash" value={txn.hash}>
          <Badge fontSize="sm" textTransform="lowercase">
            { collapseHexString(txn.hash) }
          </Badge>
        </Copyable>
      </DetailItem>
      <DetailItem label="Timestamp">
        <Copyable prompt="Copy timestamp" value={txn.timestamp}>
          <Badge fontSize="sm" textTransform="none">
            { txn.fullDatetime }
          </Badge>
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
