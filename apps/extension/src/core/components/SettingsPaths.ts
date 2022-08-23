// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { BiNetworkChart } from '@react-icons/all-files/bi/BiNetworkChart';
import { FaKey } from '@react-icons/all-files/fa/FaKey';
import { FaKeyboard } from '@react-icons/all-files/fa/FaKeyboard';
import { FaLock } from '@react-icons/all-files/fa/FaLock';
import { BiLink } from '@react-icons/all-files/bi/BiLink';
import Routes from 'core/routes';
import { FaEdit } from '@react-icons/all-files/fa/FaEdit';
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

function SettingsPaths(hasMnemonic: boolean): SettingsListItemProps[] {
  const items: SettingsListItemProps[] = [
    {
      icon: FaEdit,
      path: Routes.rename_account.path,
      title: 'Change account name',
    },
    {
      icon: FaKey,
      path: Routes.credentials.path,
      title: 'Credentials',
    },
    {
      icon: BiNetworkChart,
      path: Routes.network.path,
      title: 'Network',
    },
    {
      icon: BiLink,
      path: null,
      title: 'View on Explorer',
    },
  ];

  if (hasMnemonic) {
    items.push({
      icon: FaKeyboard,
      path: Routes.recovery_phrase.path,
      title: 'Show Recovery Phrase',
    });
  }

  items.push({
    bgColorDict: signOutSecondaryGridBgColor,
    hoverBgColorDict: signOutSecondaryGridHoverBgColor,
    icon: FaLock,
    path: Routes.wallet.path,
    textColorDict: signOutSecondaryTextColor,
    title: 'Lock wallet',
  });

  return items;
}

export default SettingsPaths;
