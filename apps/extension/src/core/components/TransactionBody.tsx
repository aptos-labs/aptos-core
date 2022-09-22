// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import React from 'react';
import {
  Badge,
  Box,
  Button,
  Center,
  Divider,
  Flex,
  HStack,
  Spinner,
  Text,
  Tooltip,
  VStack,
} from '@chakra-ui/react';
import { ExternalLinkIcon } from '@chakra-ui/icons';
import { collapseHexString } from 'core/utils/hex';
import { HexString, MaybeHexString } from 'aptos';
import { useParams } from 'react-router-dom';
import ChakraLink from 'core/components/ChakraLink';
import { useActiveAccount } from 'core/hooks/useAccounts';
import { formatAmount, formatCoin } from 'core/utils/coin';
import { useTransaction } from 'core/queries/transaction';
import { FaRegCheckCircle } from '@react-icons/all-files/fa/FaRegCheckCircle';
import { FaRegTimesCircle } from '@react-icons/all-files/fa/FaRegTimesCircle';
import Copyable from './Copyable';

const positiveAmountColor = 'green.500';
const negativeAmountColor = 'red.500';

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

function TransactionBody() {
  const { activeAccountAddress } = useActiveAccount();
  const { version } = useParams();
  const { data: txn } = useTransaction(version ? Number(version) : undefined);

  const fullDatetime = txn !== undefined
    ? new Date(txn.timestamp).toLocaleDateString('en-us', {
      day: 'numeric',
      hour: 'numeric',
      minute: 'numeric',
      month: 'short',
      year: 'numeric',
    })
    : undefined;

  const userAddress = activeAccountAddress
    && HexString.ensure(activeAccountAddress).toShortString();
  const explorerAddress = `https://explorer.aptoslabs.com/txn/${version}`;

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

  return (
    <VStack
      w="100%"
      paddingTop={8}
      px={4}
      alignItems="stretch"
    >
      <Flex alignItems="flex-start" width="100%" pb={4}>
        <Button
          fontSize="md"
          fontWeight={400}
          as="a"
          target="_blank"
          rightIcon={<ExternalLinkIcon />}
          variant="link"
          cursor="pointer"
          href={explorerAddress}
          alignSelf="end"
        >
          View on explorer
        </Button>
      </Flex>

      {
        txn === undefined
          ? (
            <Center h="100%">
              <Spinner size="xl" />
            </Center>
          )
          : null
      }
      {
        txn !== undefined
          ? (
            <>
              <DetailItem label="Version">
                <Text>{ txn.version }</Text>
              </DetailItem>
              <DetailItem label="Timestamp">
                <Copyable prompt="Copy timestamp" value={txn.timestamp}>
                  <Badge fontSize="sm" textTransform="none">
                    { fullDatetime }
                  </Badge>
                </Copyable>
              </DetailItem>
              <DetailItem label="Status">
                {
                  txn.error
                    ? (
                      <Tooltip label={txn.error.reasonDescr}>
                        <Box color="red.400">
                          <FaRegTimesCircle />
                        </Box>
                      </Tooltip>
                    )
                    : (
                      <Tooltip label="Success">
                        <Box color="green.400">
                          <FaRegCheckCircle />
                        </Box>
                      </Tooltip>
                    )
                }
              </DetailItem>
              <DetailItem label="Gas used">
                <Text>{ formatCoin(txn.gasFee * txn.gasUnitPrice, { decimals: 8 }) }</Text>
              </DetailItem>
              <DetailItem label="Gas unit price">
                <Text>{ txn.gasUnitPrice }</Text>
              </DetailItem>
            </>
          )
          : null
      }
      <Divider />
      {
        txn?.type === 'transfer'
          ? (
            <>
              <DetailItem label="Type">
                <Text>Coin transfer</Text>
              </DetailItem>
              <DetailItem label="From">
                { clickableAddress(txn.sender) }
              </DetailItem>
              <DetailItem label="To">
                { clickableAddress(txn.recipient) }
              </DetailItem>
              <DetailItem label="Amount">
                <Text>
                  { formatAmount(txn.amount, txn.coinInfo, { prefix: false }) }
                </Text>
              </DetailItem>
            </>
          )
          : null
      }
      {
        txn?.type === 'mint'
          ? (
            <>
              <DetailItem label="Type">
                <Text>Coin mint</Text>
              </DetailItem>
              <DetailItem label="To">
                { clickableAddress(txn.recipient) }
              </DetailItem>
              <DetailItem label="Amount">
                <Text>
                  { formatAmount(txn.amount, txn.coinInfo, { prefix: false }) }
                </Text>
              </DetailItem>
            </>
          )
          : null
      }
      {
        txn?.type === 'generic'
          ? (
            <>
              <DetailItem label="Function">
                <Text>{ txn.payload.function.split('::').pop() }</Text>
              </DetailItem>
              <HStack w="100%" fontSize="md" justify="space-between" alignItems="start">
                <Text as="span" fontWeight={700}> Balance changes </Text>
                <VStack alignItems="end">
                  {
                    Object.entries(txn.coinBalanceChanges[activeAccountAddress] ?? {})
                      .map(([coinType, { amount, coinInfo }]) => (
                        <Text
                          key={coinType}
                          color={amount > 0 ? positiveAmountColor : negativeAmountColor}
                        >
                          { formatAmount(amount, coinInfo) }
                        </Text>
                      ))
                  }
                </VStack>
              </HStack>

            </>
          )
          : null
      }
    </VStack>
  );
}

export default TransactionBody;
