// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  Heading, Spinner, Text, useColorMode, VStack, Wrap,
} from '@chakra-ui/react';
import React from 'react';
import { useAccountStakeBalance } from 'core/queries/account';
import numeral from 'numeral';
import { secondaryAddressFontColor } from 'core/colors';
import { useActiveAccount } from 'core/hooks/useAccounts';

function WalletAccountStake() {
  const { colorMode } = useColorMode();
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  const { activeAccountAddress } = useActiveAccount();
  // TODO: switch address to activeAccountAddress
  const {
    data: stakeBalance,
    isLoading,
  } = useAccountStakeBalance('0xb77026ce272a63b7261d20e5d0d9ca8cddd81b42b3432891668c43c03dbd1b73', {
    refetchInterval: 5000,
  });
  const stakeBalanceString = numeral(stakeBalance).format('0,0');

  return (
    <VStack alignItems="flex-start">
      <Text fontSize="sm" color={secondaryAddressFontColor[colorMode]}>My stake</Text>
      <Wrap alignItems="baseline">
        <span>
          {
            isLoading
              ? <Spinner size="md" thickness="3px" />
              : <Heading fontSize="md" as="span" wordBreak="break-word" maxW="100%">{`${stakeBalanceString}`}</Heading>
          }
          <Text pl={2} pb="2px" as="span" fontSize="md" fontWeight={600}>
            APT
          </Text>
        </span>
      </Wrap>
    </VStack>
  );
}

export default WalletAccountStake;
