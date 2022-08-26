// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import React, { LegacyRef, MouseEventHandler } from 'react';
import {
  Box,
} from '@chakra-ui/react';
import { useActiveAccount } from 'core/hooks/useAccounts';
import { Account } from 'shared/types';
import AvatarImage from 'core/accountImages';

interface AccountCircleProps {
  account?: Account;
  onClick?: MouseEventHandler<HTMLDivElement>;
  size?: number;
}

const AccountCircle = React.forwardRef((
  {
    account,
    onClick,
    size = 32,
  }: AccountCircleProps,
  ref: LegacyRef<HTMLImageElement>,
) => {
  const { activeAccountAddress } = useActiveAccount();
  const address = account?.address || activeAccountAddress;
  return (
    <Box
      borderRadius="2rem"
      cursor="pointer"
      onClick={onClick}
      ref={ref}
    >
      <AvatarImage
        size={size}
        address={address ?? ''}
      />
    </Box>
  );
});

export default AccountCircle;
