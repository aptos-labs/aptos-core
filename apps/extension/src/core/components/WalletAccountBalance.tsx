// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  Heading, Text, useColorMode, VStack,
} from '@chakra-ui/react';
import React from 'react';
import { secondaryAddressFontColor } from 'core/colors';
import { useAccountCoinBalance } from 'core/queries/account';
import numeral from 'numeral';
import useGlobalStateContext from 'core/hooks/useGlobalState';

function WalletAccountBalance() {
  const { colorMode } = useColorMode();
  const { activeAccountAddress } = useGlobalStateContext();
  const { data: coinBalance } = useAccountCoinBalance(activeAccountAddress, {
    refetchInterval: 5000,
  });
  const coinBalanceString = numeral(coinBalance).format('0,0.0000');

  return (
    <VStack>
      <Text fontSize="sm" color={secondaryAddressFontColor[colorMode]}>Account balance</Text>
      <Heading>{coinBalanceString}</Heading>
    </VStack>
  );
}

export default WalletAccountBalance;
