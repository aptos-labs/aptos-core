// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  Divider, Grid, Text, useColorMode, VStack,
} from '@chakra-ui/react';
import { secondaryTextColor } from 'core/colors';
import { useTransferFlow } from 'core/hooks/useTransferFlow';
import { formatCoin } from 'core/utils/coin';
import { collapseHexString } from 'core/utils/hex';
import React from 'react';
import Copyable from './Copyable';

export default function TransferSummary() {
  const { colorMode } = useColorMode();
  const {
    amountOctaNumber,
    estimatedGasFeeOcta,
    validRecipientAddress,
  } = useTransferFlow();
  const collapsedAddress = validRecipientAddress ? collapseHexString(validRecipientAddress) : '';
  const amountAPTString = formatCoin(amountOctaNumber);
  const estimatedGasFeeAPTString = formatCoin(estimatedGasFeeOcta, { decimals: 8 });
  const totalOctas = (amountOctaNumber || 0n) + BigInt(estimatedGasFeeOcta ?? 0);
  const totalString = formatCoin(totalOctas, { decimals: 8 });

  return (
    <VStack fontSize="md" divider={<Divider />} px={4} py={8} pb={24} gap={2}>
      <Grid gap={4} width="100%" templateColumns="80px 1fr">
        <Text color={secondaryTextColor[colorMode]}>Recipient</Text>
        <Text fontWeight={600} w="100%" textAlign="right">
          <Copyable value={validRecipientAddress}>
            {collapsedAddress}
          </Copyable>
        </Text>
      </Grid>
      <VStack width="100%">
        <Grid gap={4} width="100%" templateColumns="80px 1fr">
          <Text color={secondaryTextColor[colorMode]}>Amount</Text>
          <Text fontWeight={600} w="100%" textAlign="right">{amountAPTString}</Text>
          <Text color={secondaryTextColor[colorMode]}>Fee</Text>
          <Text fontWeight={600} w="100%" textAlign="right">{estimatedGasFeeAPTString}</Text>
        </Grid>
      </VStack>
      <Grid gap={4} width="100%" templateColumns="80px 1fr">
        <Text fontWeight={600} color={secondaryTextColor[colorMode]}>Total</Text>
        <Text fontWeight={600} w="100%" textAlign="right">{totalString}</Text>
      </Grid>
    </VStack>
  );
}
