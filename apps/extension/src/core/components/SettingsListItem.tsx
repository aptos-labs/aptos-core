// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  Center,
  Grid, Icon, Text, useColorMode, Flex,
} from '@chakra-ui/react';
import React, { useMemo } from 'react';
import { useNavigate } from 'react-router-dom';
import { useActiveAccount, useInitializedAccounts } from 'core/hooks/useAccounts';
import { settingsItemLabel } from 'core/constants';
import Browser from 'core/utils/browser';
import {
  secondaryGridHoverBgColor,
  textColor, secondaryAddressFontColor,
} from 'core/colors';

interface BgColorDictType {
  dark: string;
  light: string;
}

export interface SettingsListItemProps {
  DividerComponent?: any | undefined;
  externalLink?: string | null;
  hoverBgColorDict?: BgColorDictType;
  iconAfter?: any | undefined;
  iconBefore?: any | undefined;
  network?: any;
  path: string | null;
  textColorDict?: BgColorDictType;
  title: string;
}

export default function SettingsListItem({
  hoverBgColorDict = secondaryGridHoverBgColor,
  textColorDict = textColor,
  externalLink,
  iconAfter,
  iconBefore,
  path,
  network,
  DividerComponent,
  title,
}: SettingsListItemProps) {
  const navigate = useNavigate();
  const { colorMode } = useColorMode();
  const { activeAccount } = useActiveAccount();
  const { lockAccounts } = useInitializedAccounts();

  const gridOnClick = async () => {
    // todo: Create an enum for these titles for more typed code
    if (title === settingsItemLabel.LOCK_WALLET && activeAccount) {
      // todo: add toasts for removing the account
      // we should probably combine the toasts from the wallet drawer
      await lockAccounts();
    } else if (title === 'View on Explorer') {
      const explorerAddress = activeAccount?.address
        ? `https://explorer.devnet.aptos.dev/account/${activeAccount.address}`
        : 'https://explorer.devnet.aptos.dev';
      window.open(explorerAddress, '_blank');
    }

    if (path) {
      navigate(path);
    }

    if (externalLink) {
      Browser.redirect(externalLink);
    }
  };

  const renderTitle = useMemo(() => {
    if (title === settingsItemLabel.NETWORK) {
      return (
        <Flex gap={2}>
          Network
          <Text color={secondaryAddressFontColor[colorMode]}>{(network?.name)}</Text>
        </Flex>
      );
    }

    return title;
  }, [network, title, colorMode]);

  const templateColumns = useMemo(() => {
    if (iconBefore && iconAfter) {
      return '32px 1fr 32px';
    } if (iconBefore) {
      return '32px 1fr';
    }
    return '1fr 32px';
  }, [iconBefore, iconAfter]);

  return (
    <>
      <Grid
        templateColumns={templateColumns}
        p={4}
        width="100%"
        cursor="pointer"
        onClick={gridOnClick}
        gap={2}
        borderRadius=".5rem"
        _hover={{
          bgColor: hoverBgColorDict[colorMode],
        }}
      >
        {iconBefore ? (
          <Center width="100%">
            <Icon
              fontSize="xl"
              borderColor={textColorDict[colorMode]}
              color={textColorDict[colorMode]}
              as={iconBefore}
            />
          </Center>
        ) : null}
        <Text
          color={textColorDict[colorMode]}
          fontWeight={600}
          fontSize="md"
        >
          {renderTitle}
        </Text>
        {iconAfter
          ? (
            <Center width="100%">
              <Icon
                fontSize="xl"
                borderColor={textColorDict[colorMode]}
                color={secondaryAddressFontColor[colorMode]}
                as={iconAfter}
              />
            </Center>
          ) : null}
      </Grid>
      {DividerComponent ? <DividerComponent /> : null}
    </>
  );
}
