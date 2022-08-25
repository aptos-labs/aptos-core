// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import React, { LegacyRef, MouseEventHandler } from 'react';
import {
  Box,
} from '@chakra-ui/react';
import { useActiveAccount } from 'core/hooks/useAccounts';
import AvatarImage from 'core/accountImages';

interface ButtonProps {
  height?: number;
  onClick?: MouseEventHandler<HTMLDivElement>;
  width?: number;
}

const AccountCircle = React.forwardRef((
  { height = 32, onClick, width = 32 }: ButtonProps,
  ref: LegacyRef<HTMLImageElement>,
) => {
  const { activeAccountAddress } = useActiveAccount();
  return (
    <Box
      height={`${height}px`}
      width={`${width}px`}
      borderRadius="2rem"
      cursor="pointer"
      onClick={onClick}
      ref={ref}
    >
      <AvatarImage
        size={32}
        address={activeAccountAddress ?? ''}
      />
    </Box>
  );
});

export default AccountCircle;
