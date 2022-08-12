// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  Heading, Text, useColorMode, VStack,
} from '@chakra-ui/react';
import React from 'react';
import { useAccountCoinBalance } from 'core/queries/account';
import numeral from 'numeral';
import useGlobalStateContext from 'core/hooks/useGlobalState';
import { secondaryAddressFontColor } from 'core/colors';

function WalletAccountBalance() {
  const { colorMode } = useColorMode();
  const { activeAccountAddress } = useGlobalStateContext();
  const { data: coinBalance } = useAccountCoinBalance(activeAccountAddress, {
    refetchInterval: 5000,
  });
  const coinBalanceString = numeral(coinBalance).format('0,0');

  return (
    <VStack px={4} alignItems="left">
      <Text fontSize="sm" color={secondaryAddressFontColor[colorMode]}>Account balance</Text>
      <Heading>{coinBalanceString}</Heading>
    </VStack>
  );
}

export default WalletAccountBalance;
