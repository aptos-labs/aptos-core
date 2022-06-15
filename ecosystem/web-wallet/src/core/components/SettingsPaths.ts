// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { BiNetworkChart } from 'react-icons/bi';
import { FaKey } from 'react-icons/fa';
import { GoSignOut } from 'react-icons/go';
import { SettingsListItemProps } from './SettingsListItem';

export const signOutSecondaryGridBgColor = {
  dark: 'red.500',
  light: 'red.500',
};

export const signOutSecondaryGridHoverBgColor = {
  dark: 'red.600',
  light: 'red.400',
};

export const signOutSecondaryTextColor = {
  dark: 'white',
  light: 'white',
};

const SettingsPaths: SettingsListItemProps[] = [
  {
    icon: FaKey,
    path: '/settings/credentials',
    title: 'Credentials',
  },
  {
    icon: BiNetworkChart,
    path: '/settings/network',
    title: 'Network',
  },
  {
    bgColorDict: signOutSecondaryGridBgColor,
    hoverBgColorDict: signOutSecondaryGridHoverBgColor,
    icon: GoSignOut,
    path: '/',
    textColorDict: signOutSecondaryTextColor,
    title: 'Sign out',
  },
];

export default SettingsPaths;
