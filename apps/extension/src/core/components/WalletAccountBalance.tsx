// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  Heading, Text, useColorMode, VStack,
} from '@chakra-ui/react';
import React from 'react';
import { secondaryAddressFontColor } from 'core/colors';
import { useAccountCoinBalance } from 'core/queries/account';
import numeral from 'numeral';
import { useWalletState } from 'core/hooks/useWalletState';

function WalletAccountBalance() {
  const { colorMode } = useColorMode();
  const { aptosAccount } = useWalletState();
  const { data: coinBalance } = useAccountCoinBalance({
    address: aptosAccount?.address().hex(),
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
