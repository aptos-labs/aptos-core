// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  Heading, Text, useColorMode, VStack,
} from '@chakra-ui/react';
import React from 'react';
import { seconaryAddressFontColor } from 'core/components/WalletHeader';
import { getTestCoinTokenBalanceFromAccountResources, useAccountResources } from 'core/queries/account';
import numeral from 'numeral';

function WalletAccountBalance() {
  const { colorMode } = useColorMode();
  const {
    data: accountResources,
  } = useAccountResources();

  const tokenBalance = getTestCoinTokenBalanceFromAccountResources({ accountResources });
  const tokenBalanceString = numeral(tokenBalance).format('0,0.0000');

  return (
    <VStack>
      <Text fontSize="sm" color={seconaryAddressFontColor[colorMode]}>Account balance</Text>
      <Heading>{tokenBalanceString}</Heading>
    </VStack>
  );
}

export default WalletAccountBalance;
