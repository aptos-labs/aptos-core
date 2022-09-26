// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { Routes } from './routes';

// Text

export const textColor = {
  dark: 'white',
  light: 'black',
};

export const secondaryAddressFontColor = {
  dark: 'gray.400',
  light: 'gray.500',
};

export const secondaryTextColor = {
  dark: 'gray.400',
  light: 'gray.500',
};

export const secondaryErrorMessageColor = {
  dark: 'red.200',
  light: 'red.500',
};

export const secondaryExtensionBodyTextColor = {
  dark: 'gray.400',
  light: 'gray.400',
};

export const timestampColor = {
  dark: 'gray.500',
  light: 'gray.500',
};

// Button

export const secondaryButtonColor = {
  dark: 'gray.600',
  light: 'gray.100',
};

// Background

// color hex code come from https://chakra-ui.com/docs/styled-system/theme#green
// for some reason green.100 and green.400 does not work
// TODO investigate why
export const checkCircleSuccessBg = {
  dark: '#C6F6D5',
  light: '#48BB78',
};

export const removeButtonBg = {
  dark: '#D76D61',
  light: '#D76D61',
};

export const secondaryBgColor = {
  dark: 'gray.900',
  light: 'white',
};

export const walletHeaderBgColor = {
  dark: 'gray.600',
  light: 'gray.100',
};

export const accountViewBgColor = {
  dark: 'gray.600',
  light: 'gray.200',
};

export const secondaryBorderColor = {
  dark: 'whiteAlpha.200',
  light: 'blackAlpha.200',
};

export const buttonBorderColor = {
  dark: 'gray.700',
  light: 'gray.200',
};

export const mnemonicBorderColor = {
  dark: 'gray.700',
  light: 'white',
};

export const secondaryHeaderInputBgColor = {
  dark: 'gray.700',
  light: 'gray.100',
};

export const secondaryHeaderInputHoverBgColor = {
  dark: 'gray.600',
  light: 'gray.200',
};

export const secondaryHeaderBgColor = {
  dark: 'gray.700',
  light: 'gray.200',
};

export const secondaryHoverBgColor = {
  dark: 'gray.700',
  light: 'gray.200',
};

export const secondaryBackButtonBgColor = {
  dark: 'gray.600',
  light: 'gray.100',
};

export const secondaryGridHoverBgColor = {
  dark: 'gray.600',
  light: 'gray.200',
};

export const secondaryGridBgColor = {
  dark: 'gray.700',
  light: 'gray.100',
};

export const secondaryDisabledNetworkBgColor = {
  dark: 'gray.800',
  light: 'gray.50',
};

// Other

export const secondaryDividerColor = {
  dark: 'whiteAlpha.300',
  light: 'gray.200',
};

export const secondaryWalletHomeCardBgColor = {
  dark: 'gray.800',
  light: 'gray.50',
};

export const iconBgColor = {
  dark: 'gray.700',
  light: 'gray.100',
};

export const iconColor = {
  dark: 'white',
  light: 'gray.600',
};

export const permissionRequestLayoutBgColor = {
  dark: 'gray.900',
  light: 'white',
};

export const permissionRequestBgColor = {
  dark: 'gray.900',
  light: 'rgb(247, 247, 247)',
};

export const permissionRequestTileBgColor = {
  dark: '#2e3038',
  light: 'white',
};

export const permissionRequestLoadingOverlayColor = {
  dark: '#2e3038b5',
  light: '#ffffffb5',
};

// Wallet

export const assetSecondaryBgColor = {
  dark: 'gray.800',
  light: 'gray.100',
};

export const walletBgColor = (pathname: string) => {
  switch (pathname) {
    case Routes.wallet.path:
      return 'navy.900';
    default:
      return undefined;
  }
};

export const walletTextColor = (pathname: string) => {
  switch (pathname) {
    case Routes.wallet.path:
      return 'white';
    default:
      return undefined;
  }
};

export const secondaryButtonBgColor = {
  dark: 'gray.800',
  light: 'white',
};

export const customColors = {
  green: {
    100: '#EDF9F8',
    200: '#D8EEEC',
    300: '#B8E0DD',
    400: '#95D0CC',
    // BRAND
    500: '#70C0BA',
    600: '#4EB1AA',
    700: '#49A69F',
    800: '#3E8E88',
    900: '#306E69',
  },
  navy: {
    100: '#F1F2F3',
    200: '#DEE1E3',
    300: '#C2C7CC',
    400: '#B7BCBD',
    500: '#A1A9AF',
    600: '#828C95',
    700: '#4d5c6d',
    800: '#324459',
    // primary
    900: '#172B45',
  },
  orange: {
    200: '#F3A845',
  },
  salmon: {
    100: '#FFBDBD',
    200: '#FF9E9E',
    300: '#FF8A8A',
    400: '#FF7575',
    50: '#FFF0F0',
    // primary
    500: '#FF5F5F',
    600: '#E15656',
    700: '#953232',
    800: '#6F2525',
    900: '#491818',
  },
};

export const stepBorderColor = {
  dark: customColors.navy[100],
  light: customColors.navy[900],
};
