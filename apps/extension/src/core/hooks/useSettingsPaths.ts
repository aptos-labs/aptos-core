// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { Divider } from '@chakra-ui/react';
import { AiFillQuestionCircle } from '@react-icons/all-files/ai/AiFillQuestionCircle';
import { FiChevronRight } from '@react-icons/all-files/fi/FiChevronRight';
import { BsShieldFill } from '@react-icons/all-files/bs/BsShieldFill';
import { FaLock } from '@react-icons/all-files/fa/FaLock';
import { MdWifiTethering } from '@react-icons/all-files/md/MdWifiTethering';
import { SettingsListItemProps } from 'core/components/SettingsListItem';
import { settingsItemLabel } from 'core/constants';
import useExplorerAddress from 'core/hooks/useExplorerAddress';
import Routes from 'core/routes';
import { Account } from 'shared/types';
import { useMemo } from 'react';

export default function useSettingsPaths(account: Account): SettingsListItemProps[] {
  const getExplorerAddress = useExplorerAddress();

  return useMemo(() => {
    const items: SettingsListItemProps[] = [
      {
        iconAfter: FiChevronRight,
        iconBefore: MdWifiTethering,
        path: Routes.network.path,
        title: settingsItemLabel.NETWORK,
      },
      {
        DividerComponent: Divider,
        iconAfter: FiChevronRight,
        iconBefore: BsShieldFill,
        path: Routes.security_privacy.path,
        title: 'Security and Privacy',
      },
      {
        externalLink: 'https://discord.com/invite/petrawallet',
        iconBefore: AiFillQuestionCircle,
        path: null,
        title: settingsItemLabel.HELP_SUPPORT,
      },
      {
        DividerComponent: Divider,
        iconBefore: FaLock,
        path: Routes.wallet.path,
        title: settingsItemLabel.LOCK_WALLET,
      },
      {
        iconAfter: FiChevronRight,
        path: Routes.manage_account.path,
        title: settingsItemLabel.MANAGE_ACCOUNT,
      },
      {
        externalLink: getExplorerAddress(`account/${account.address}`),
        iconAfter: FiChevronRight,
        path: null,
        title: settingsItemLabel.EXPLORER,
      },
      {
        iconAfter: FiChevronRight,
        path: Routes.switchAccount.path,
        title: settingsItemLabel.SWITCH_ACCOUNT,
      },
      {
        iconAfter: FiChevronRight,
        path: Routes.remove_account.path,
        textColorDict: {
          dark: 'red.400',
          light: 'red.400',
        },
        title: settingsItemLabel.REMOVE_ACCOUNT,
      },
    ];

    return items;
  }, [account, getExplorerAddress]);
}
