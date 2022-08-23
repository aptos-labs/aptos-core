// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import React from 'react';
import {
  Badge,
  Box,
  Button,
  Center,
  HStack,
  Spinner,
  Text,
  Tooltip,
  VStack,
} from '@chakra-ui/react';
import { FaRegCheckCircle } from '@react-icons/all-files/fa/FaRegCheckCircle';
import { FaRegTimesCircle } from '@react-icons/all-files/fa/FaRegTimesCircle';
import { ExternalLinkIcon } from '@chakra-ui/icons';
import { collapseHexString } from 'core/utils/hex';
import { HexString, MaybeHexString } from 'aptos';
import { useParams } from 'react-router-dom';
import ChakraLink from 'core/components/ChakraLink';
import { useActiveAccount } from 'core/hooks/useAccounts';
import useTransactionDetails from 'core/hooks/useTransactionDetails';
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

function TransactionBody() {
  const { activeAccountAddress } = useActiveAccount();
  const { version } = useParams();

  const txn = useTransactionDetails(version ? Number(version) : undefined);
  const userAddress = activeAccountAddress
    && HexString.ensure(activeAccountAddress).toShortString();
  const explorerAddress = `https://explorer.devnet.aptos.dev/txn/${version}`;

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
      <Button
        fontSize="sm"
        fontWeight={400}
        as="a"
        target="_blank"
        rightIcon={<ExternalLinkIcon />}
        variant="unstyled"
        cursor="pointer"
        href={explorerAddress}
        alignSelf="end"
      >
        View on explorer
      </Button>
      {
        !txn
          ? (
            <Center h="100%">
              <Spinner size="xl" />
            </Center>
          )
          : (
            <>
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
                <Text>{ txn.gasUsed }</Text>
              </DetailItem>
            </>
          )
      }
    </VStack>
  );
}

export default TransactionBody;
