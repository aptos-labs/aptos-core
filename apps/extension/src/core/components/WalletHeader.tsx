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
  VStack,
  useClipboard,
} from '@chakra-ui/react';
import { Routes } from 'core/routes';
import { ArrowBackIcon } from '@chakra-ui/icons';
import {
  secondaryBackButtonBgColor, secondaryBorderColor, walletBgColor, walletTextColor,
} from 'core/colors';
import { useLocation, useNavigate } from 'react-router-dom';
import AccountCircle from 'core/components/AccountCircle';
import { useActiveAccount } from 'core/hooks/useAccounts';
import collapseHexString from 'core/utils/hex';
import { BiCopy } from '@react-icons/all-files/bi/BiCopy';

interface WalletHeaderProps {
  showAccountCircle?: boolean;
  showBackButton?: boolean;
  title?: string
}

export default function WalletHeader({
  showAccountCircle,
  showBackButton,
  title,
}: WalletHeaderProps) {
  const { pathname } = useLocation();
  const navigate = useNavigate();
  const { colorMode } = useColorMode();
  const { activeAccount } = useActiveAccount();
  const { activeAccountAddress } = useActiveAccount();
  const { hasCopied, onCopy } = useClipboard(activeAccountAddress ?? '');

  const borderBottomColor = useMemo(() => {
    switch (pathname) {
      case Routes.wallet.path:
        return 'transparent';
      default:
        return secondaryBorderColor[colorMode];
    }
  }, [colorMode, pathname]);

  const walletNameAndAddress = useMemo(() => {
    switch (pathname) {
      case Routes.wallet.path:
      case Routes.gallery.path:
      case Routes.stake.path:
      case Routes.activity.path:
        return (
          <Tooltip label={hasCopied ? 'Copied!' : 'Copy Address'} closeDelay={300}>
            <HStack spacing={0} width="100%" cursor="pointer" onClick={onCopy}>
              <Text as="span" fontSize="sm" fontWeight={400}>
                {`${activeAccount.name?.toString() || 'Account'} (${collapseHexString(activeAccount.address, 8, true)})`}
              </Text>
              <Box>
                <IconButton
                  height="16px"
                  width="16px"
                  fontSize="16px"
                  size="xs"
                  icon={<BiCopy />}
                  aria-label="Copy Address"
                  bg="clear"
                  _focus={{ boxShadow: 'none' }}
                  _active={{
                    bg: 'none',
                    transform: 'scale(0.90)',
                  }}
                  variant="none"
                />
              </Box>
            </HStack>
          </Tooltip>
        );
      default:
        return undefined;
    }
  }, [pathname, hasCopied, activeAccount.name, activeAccount.address, onCopy]);

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
        height="73px"
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
          <VStack spacing={0} alignItems="flex-start" width="100%">
            <Text fontSize={20} fontWeight={600}>
              {title}
            </Text>
            {walletNameAndAddress}
          </VStack>
        </HStack>
        <HStack spacing={4}>
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
