// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import React, { useMemo } from 'react';
import {
  Box,
  HStack,
  Tooltip,
  Text,
  useColorMode,
  IconButton,
} from '@chakra-ui/react';
import { Routes } from 'core/routes';
import { ArrowBackIcon } from '@chakra-ui/icons';
import {
  secondaryBackButtonBgColor, secondaryBorderColor, walletBgColor, walletTextColor,
} from 'core/colors';
import { useLocation, useNavigate } from 'react-router-dom';
import AccountCircle from 'core/components/AccountCircle';

interface WalletHeaderProps {
  accessoryButton?: React.ReactNode,
  showAccountCircle?: boolean;
  showBackButton?: boolean;
  title?: string
}

export default function WalletHeader({
  accessoryButton,
  showAccountCircle,
  showBackButton,
  title,
}: WalletHeaderProps) {
  const { pathname } = useLocation();
  const navigate = useNavigate();
  const { colorMode } = useColorMode();

  const borderBottomColor = useMemo(() => {
    switch (pathname) {
      case '/wallet':
        return 'transparent';
      default:
        return secondaryBorderColor[colorMode];
    }
  }, [colorMode, pathname]);

  const bgColor = useMemo(() => walletBgColor(pathname), [pathname]);
  const textColor = useMemo(() => walletTextColor(pathname), [pathname]);

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
        borderBottomColor={borderBottomColor}
        borderBottomWidth="1px"
        justifyContent="space-between"
        padding={4}
        bgColor={bgColor}
        color={textColor}
      >
        <HStack spacing={4}>
          {
            (showBackButton) ? (
              <IconButton
                size="lg"
                aria-label="back"
                colorScheme="teal"
                icon={<ArrowBackIcon fontSize={26} />}
                variant="filled"
                onClick={backOnClick}
                bgColor={secondaryBackButtonBgColor[colorMode]}
                borderRadius="1rem"
              />
            ) : null
          }
          <Text fontSize={22} fontWeight={600}>
            {title}
          </Text>
        </HStack>
        <HStack spacing={4}>
          {accessoryButton}
          {showAccountCircle ? (
            <Tooltip label="Switch wallet" closeDelay={300}>
              <AccountCircle onClick={() => navigate(Routes.switchAccount.path)} />
            </Tooltip>
          ) : null}
        </HStack>
      </HStack>
    </Box>
  );
}
