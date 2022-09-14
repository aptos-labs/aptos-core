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
      return 'navy.800';
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
