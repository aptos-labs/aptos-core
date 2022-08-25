/* eslint-disable @typescript-eslint/no-unused-vars */
// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { VStack } from '@chakra-ui/react';
import { useActiveAccount } from 'core/hooks/useAccounts';
import { useAccountStakeInfo } from 'core/queries/account';
import React from 'react';

export default function StakeBody() {
  const { activeAccountAddress } = useActiveAccount();
  const { data: stakeInfo } = useAccountStakeInfo('0xb77026ce272a63b7261d20e5d0d9ca8cddd81b42b3432891668c43c03dbd1b73');

  return (
    <VStack />
  );
}
