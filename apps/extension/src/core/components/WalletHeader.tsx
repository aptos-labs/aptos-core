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
import { secondaryBorderColor } from 'core/colors';
import { useNavigate } from 'react-router-dom';
import AccountDrawer from 'core/components/AccountDrawer';
import useGlobalStateContext from 'core/hooks/useGlobalState';
import indexColor from 'core/accountColors';

interface ButtonProps {
  onClick: MouseEventHandler<HTMLDivElement>;
}

const AccountCircle = React.forwardRef((
  { onClick }: ButtonProps,
  ref: LegacyRef<HTMLDivElement>,
) => {
  const { activeAccount } = useGlobalStateContext();
  const color = indexColor(activeAccount?.styleIndex ?? 0);
  return (
    <Box
      height="40px"
      width="40px"
      background={color}
      borderRadius="2rem"
      cursor="pointer"
      onClick={onClick}
      ref={ref}
    />
  );
});

function BackButton({ onClick }: ButtonProps) {
  return (
    <Box
      height="44px"
      width="44px"
      background="#F2F4F8"
      borderRadius="0.5rem"
      cursor="pointer"
      onClick={onClick}
    >
      <ChevronLeftIcon color="#333333" width="100%" height="100%" />
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

  return (
    <Box>
      <HStack
        maxW="100%"
        width="100%"
        py={4}
        height="84px"
        borderBottomColor={secondaryBorderColor[colorMode]}
        borderBottomWidth="1px"
        justifyContent="space-between"
        padding={4}
      >
        <HStack>
          {(showBackButton)
            ? (
              <BackButton onClick={() => navigate(-1)} />
            )
            : null}
          <Text fontSize="xl" fontWeight="semibold">
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
