// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  Heading, Spinner, Text, useColorMode, VStack, Wrap,
} from '@chakra-ui/react';
import React from 'react';
import { useAccountStakeBalance } from 'core/queries/account';
import { secondaryAddressFontColor } from 'core/colors';
import { useActiveAccount } from 'core/hooks/useAccounts';
import { APTOS_UNIT, formatCoin } from 'core/utils/coin';

function WalletAccountStake() {
  const { colorMode } = useColorMode();
  const { activeAccountAddress } = useActiveAccount();
  const {
    data: stakeBalance,
    isLoading,
  } = useAccountStakeBalance(activeAccountAddress, {
    refetchInterval: 5000,
  });
  const stakeBalanceString = formatCoin(stakeBalance, { includeUnit: false });

  return (
    <VStack alignItems="flex-start">
      <Text fontSize="sm" color={secondaryAddressFontColor[colorMode]}>Stake balance</Text>
      <Wrap alignItems="baseline">
        <span>
          {
            isLoading
              ? <Spinner size="md" thickness="3px" />
              : <Heading fontSize="md" as="span" wordBreak="break-word" maxW="100%">{stakeBalanceString}</Heading>
          }
          <Text pl={2} pb="2px" as="span" fontSize="md" fontWeight={600}>
            {APTOS_UNIT}
          </Text>
        </span>
      </Wrap>
    </VStack>
  );
}

export default WalletAccountStake;
