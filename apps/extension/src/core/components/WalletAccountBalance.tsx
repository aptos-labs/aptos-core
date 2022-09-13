// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  Heading, Spinner, Text, VStack, Wrap,
} from '@chakra-ui/react';
import React from 'react';
import { useAccountOctaCoinBalance } from 'core/queries/account';
import { useActiveAccount } from 'core/hooks/useAccounts';
import { APTOS_UNIT, formatCoin, OCTA_UNIT } from 'core/utils/coin';

function WalletAccountBalance() {
  const { activeAccountAddress } = useActiveAccount();

  const {
    data: coinBalance,
    isLoading,
  } = useAccountOctaCoinBalance(activeAccountAddress, {
    refetchInterval: 5000,
  });

  const coinBalanceString = formatCoin(coinBalance, {
    includeUnit: false,
    paramUnitType: OCTA_UNIT,
    returnUnitType: APTOS_UNIT,
  });

  return (
    <VStack alignItems="left">
      <Wrap alignItems="baseline">
        <span>
          {
            isLoading
              ? <Spinner size="md" thickness="3px" />
              : <Heading as="span" wordBreak="break-word" maxW="100%">{`${coinBalanceString}`}</Heading>
          }
          <Text pl={2} pb="2px" as="span" fontSize="xl" fontWeight={600}>
            {APTOS_UNIT}
          </Text>
        </span>
      </Wrap>
    </VStack>
  );
}

export default WalletAccountBalance;
