// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  Heading, Text, useColorMode, VStack,
} from '@chakra-ui/react';
import React from 'react';
import { secondaryAddressFontColor } from 'core/colors';
import { useAccountCoinBalance } from 'core/queries/account';
import numeral from 'numeral';

function WalletAccountBalance() {
  const { colorMode } = useColorMode();
  const { data: coinBalance } = useAccountCoinBalance({ refetchInterval: 5000 });
  const coinBalanceString = numeral(coinBalance).format('0,0.0000');

  return (
    <VStack>
      <Text fontSize="sm" color={secondaryAddressFontColor[colorMode]}>Account balance</Text>
      <Heading>{coinBalanceString}</Heading>
    </VStack>
  );
}

export default WalletAccountBalance;
