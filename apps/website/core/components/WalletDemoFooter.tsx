// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  Box,
  Center, IconButton, SimpleGrid, Text, useColorMode,
} from '@chakra-ui/react';
import { IoMdImage } from '@react-icons/all-files/io/IoMdImage';
import { RiCopperCoinFill } from '@react-icons/all-files/ri/RiCopperCoinFill';
import { RiFileListFill } from '@react-icons/all-files/ri/RiFileListFill';
import React from 'react';
import { SettingsIcon } from '@chakra-ui/icons';

export const secondaryBorderColor = {
  dark: 'whiteAlpha.200',
  light: 'blackAlpha.200',
};

const secondaryIconColor = {
  dark: 'whiteAlpha.500',
  light: 'blackAlpha.500',
};

const secondaryIconUnpressedColor = {
  dark: 'teal.400',
  light: 'teal.400',
};

const Routes = Object.freeze({
  account: {
    path: '/accounts/:address',
  },
  activity: {
    path: '/activity',
  },
  addAccount: {
    path: '/add-account',
  },
  addNetwork: {
    path: '/settings/add-network',
  },
  createAccount: {
    path: '/create-account',
  },
  createWallet: {
    path: '/create-wallet',
  },
  credentials: {
    path: '/settings/credentials',
  },
  gallery: {
    path: '/gallery',
  },
  help: {
    path: '/help',
  },
  importWalletMnemonic: {
    path: '/import/mnemonic',
  },
  importWalletPrivateKey: {
    path: '/import/private-key',
  },
  login: {
    path: '/',
  },
  network: {
    path: '/settings/network',
  },
  noWallet: {
    path: '/no-wallet',
  },
  password: {
    path: '/password',
  },
  recovery_phrase: {
    path: '/settings/recovery_phrase',
  },
  rename_account: {
    path: '/settings/rename_account',
  },
  settings: {
    path: '/settings',
  },
  token: {
    path: '/tokens/:id',
  },
  transaction: {
    path: '/transactions/:version',
  },
  wallet: {
    path: '/wallet',
  },
} as const);

interface WalletFooterProps {
  pathname: string;
}

export default function WalletFooter({
  pathname,
}: WalletFooterProps) {
  const { colorMode } = useColorMode();

  return (
    <Center
      maxW="100%"
      width="100%"
      borderTopWidth="1px"
      borderTopColor={secondaryBorderColor[colorMode]}
    >
      <SimpleGrid width="100%" gap={4} columns={4}>
        <Center flexDir="column" width="100%">
          <Box display="flex" flexDir="column" alignItems="center">
            <IconButton
              color={(pathname.includes(Routes.wallet.path))
                ? secondaryIconUnpressedColor[colorMode]
                : secondaryIconColor[colorMode]}
              variant="unstyled"
              size="md"
              aria-label="Wallet"
              fontSize="xl"
              icon={<RiCopperCoinFill size={26} />}
              display="flex"
              height="20px"
            />
            <Text
              fontWeight={600}
              color={(pathname.includes(Routes.wallet.path))
                ? secondaryIconUnpressedColor[colorMode]
                : secondaryIconColor[colorMode]}
              pt={1}
              fontSize="10px"
            >
              Home
            </Text>
          </Box>
        </Center>
        <Center flexDir="column" width="100%">
          <Box display="flex" flexDir="column" alignItems="center">
            <IconButton
              color={(pathname.includes(Routes.gallery.path) || pathname.includes('/tokens'))
                ? secondaryIconUnpressedColor[colorMode]
                : secondaryIconColor[colorMode]}
              variant="unstyled"
              size="md"
              aria-label="Gallery"
              icon={<IoMdImage size={26} />}
              fontSize="xl"
              display="flex"
              height="20px"
              isDisabled
            />
            <Text
              fontWeight={600}
              color={(pathname.includes(Routes.gallery.path) || pathname.includes('/tokens'))
                ? secondaryIconUnpressedColor[colorMode]
                : secondaryIconColor[colorMode]}
              pt={1}
              fontSize="10px"
            >
              Library
            </Text>
          </Box>
        </Center>
        <Center flexDir="column" width="100%">
          <Box display="flex" flexDir="column" alignItems="center">
            <IconButton
              color={(pathname.includes(Routes.activity.path) || pathname.includes('/transactions'))
                ? secondaryIconUnpressedColor[colorMode]
                : secondaryIconColor[colorMode]}
              variant="unstyled"
              size="md"
              aria-label="Activity"
              icon={<RiFileListFill size={26} />}
              fontSize="xl"
              display="flex"
              height="20px"
              isDisabled
            />
            <Text
              fontWeight={600}
              color={(pathname.includes(Routes.activity.path) || pathname.includes('/transactions'))
                ? secondaryIconUnpressedColor[colorMode]
                : secondaryIconColor[colorMode]}
              pt={1}
              fontSize="10px"
            >
              Activity
            </Text>
          </Box>
        </Center>
        <Center flexDir="column" width="100%">
          <Box display="flex" flexDir="column" alignItems="center">
            <IconButton
              color={(pathname.includes(Routes.settings.path))
                ? secondaryIconUnpressedColor[colorMode]
                : secondaryIconColor[colorMode]}
              variant="unstyled"
              size="md"
              aria-label="Account"
              icon={<SettingsIcon fontSize={24} />}
              fontSize="xl"
              display="flex"
              height="20px"
              isDisabled
            />
            <Text
              fontWeight={600}
              color={(pathname.includes(Routes.settings.path))
                ? secondaryIconUnpressedColor[colorMode]
                : secondaryIconColor[colorMode]}
              pt={1}
              fontSize="10px"
            >
              Settings
            </Text>
          </Box>
        </Center>
      </SimpleGrid>
    </Center>
  );
}
