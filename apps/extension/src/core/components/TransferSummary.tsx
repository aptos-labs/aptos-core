// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  Divider, Grid, Text, useColorMode, VStack,
} from '@chakra-ui/react';
import { secondaryTextColor } from 'core/colors';
import { collapseHexString } from 'core/utils/hex';
import numeral from 'numeral';
import React from 'react';
import Copyable from './Copyable';

interface TransferSummaryProps {
  amount?: number;
  estimatedGasFee?: number;
  recipient?: string;
  unit?: string;
}

export default function TransferSummary({
  amount,
  estimatedGasFee,
  recipient,
  unit,
}: TransferSummaryProps) {
  const { colorMode } = useColorMode();
  const collapsedAddress = recipient ? collapseHexString(recipient) : '';

  const amountString = `${numeral(amount).format('0,0')} ${unit}`;
  const estimatedGasFeeString = `${estimatedGasFee} ${unit}`;
  const total = (amount || 0) + (estimatedGasFee || 0);
  const totalString = `${numeral(total).format('0,0')} ${unit}`;

  return (
    <VStack fontSize="md" divider={<Divider />} px={4} py={8} pb={24} gap={2}>
      <Grid gap={4} width="100%" templateColumns="80px 1fr">
        <Text color={secondaryTextColor[colorMode]}>Recipient</Text>
        <Text fontWeight={600} w="100%" textAlign="right">
          <Copyable value={recipient}>
            {collapsedAddress}
          </Copyable>
        </Text>
      </Grid>
      <VStack width="100%">
        <Grid gap={4} width="100%" templateColumns="80px 1fr">
          <Text color={secondaryTextColor[colorMode]}>Amount</Text>
          <Text fontWeight={600} w="100%" textAlign="right">{amountString}</Text>
          <Text color={secondaryTextColor[colorMode]}>Fee</Text>
          <Text fontWeight={600} w="100%" textAlign="right">{estimatedGasFeeString}</Text>
        </Grid>
      </VStack>
      <Grid gap={4} width="100%" templateColumns="80px 1fr">
        <Text fontWeight={600} color={secondaryTextColor[colorMode]}>Total</Text>
        <Text fontWeight={600} w="100%" textAlign="right">{totalString}</Text>
      </Grid>
    </VStack>
  );
}
