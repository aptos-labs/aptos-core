// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import React, { LegacyRef, MouseEventHandler } from 'react';
import {
  Box,
  HStack,
  Tooltip,
  Text,
  useColorMode,
  useDisclosure,
} from '@chakra-ui/react';
import { ChevronLeftIcon } from '@chakra-ui/icons';
import { secondaryBorderColor, secondaryHoverBgColor, textColor } from 'core/colors';
import { useNavigate } from 'react-router-dom';
import AccountDrawer from 'core/components/AccountDrawer';
import { useActiveAccount } from 'core/hooks/useAccounts';
import AvatarImage from 'core/accountImages';

interface ButtonProps {
  onClick: MouseEventHandler<HTMLDivElement>;
}

const AccountCircle = React.forwardRef((
  { onClick }: ButtonProps,
  ref: LegacyRef<HTMLImageElement>,
) => {
  const { activeAccountAddress } = useActiveAccount();
  return (
    <Box
      height="32px"
      width="32px"
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

function BackButton({ onClick }: ButtonProps) {
  const { colorMode } = useColorMode();
  return (
    <Box
      height="36px"
      width="36px"
      background={secondaryHoverBgColor[colorMode]}
      borderRadius="0.5rem"
      cursor="pointer"
      onClick={onClick}
    >
      <ChevronLeftIcon color={textColor[colorMode]} width="100%" height="100%" />
    </Box>
  );
}

interface WalletHeaderProps {
  accessoryButton?: React.ReactNode,
  showBackButton?: boolean;
  title?: string
}

export default function WalletHeader({
  accessoryButton,
  showBackButton,
  title,
}: WalletHeaderProps) {
  const navigate = useNavigate();
  const { colorMode } = useColorMode();
  const { isOpen, onClose, onOpen } = useDisclosure();

  const backOnClick = () => {
    navigate(-1);
  };

  return (
    <Box>
      <HStack
        maxW="100%"
        width="100%"
        py={4}
        height="70px"
        borderBottomColor={secondaryBorderColor[colorMode]}
        borderBottomWidth="1px"
        justifyContent="space-between"
        padding={4}
      >
        <HStack>
          {(showBackButton)
            ? (
              <BackButton onClick={backOnClick} />
            )
            : null}
          <Text fontSize={20} fontWeight={500}>
            {title}
          </Text>
        </HStack>
        <HStack spacing={4}>
          {accessoryButton}
          <Tooltip label="Switch wallet" closeDelay={300}>
            <AccountCircle onClick={onOpen} />
          </Tooltip>
        </HStack>
      </HStack>
      <AccountDrawer isOpen={isOpen} onClose={onClose} />
    </Box>

  );
}
