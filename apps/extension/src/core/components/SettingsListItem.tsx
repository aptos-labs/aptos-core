// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  Center,
  Grid, Icon, Text, useColorMode, Flex, VStack,
} from '@chakra-ui/react';
import React, { useCallback, useMemo } from 'react';
import { useActiveAccount, useInitializedAccounts, useUnlockedAccounts } from 'core/hooks/useAccounts';
import { settingsItemLabel } from 'core/constants';
import {
  secondaryGridHoverBgColor,
  textColor, secondaryAddressFontColor,
} from 'core/colors';
import { Routes } from 'core/routes';
import { useNavigate } from 'react-router-dom';
import ChakraLink from './ChakraLink';
import { removeAccountToast } from './Toast';

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
  const { colorMode } = useColorMode();
  const { activeAccount } = useActiveAccount();
  const { lockAccounts } = useInitializedAccounts();
  const { removeAccount } = useUnlockedAccounts();
  const navigate = useNavigate();

  const gridOnClick = useCallback(async () => {
    // todo: Create an enum for these titles for more typed code
    if (title === settingsItemLabel.LOCK_WALLET && activeAccount) {
      // todo: add toasts for removing the account
      // we should probably combine the toasts from the wallet drawer
      await lockAccounts();
    } else if (title === settingsItemLabel.REMOVE_ACCOUNT) {
      await removeAccount(activeAccount.address);
      const removedAddress = `${activeAccount.address.slice(0, 4)}...${activeAccount.address.slice(62)}`;
      removeAccountToast(`Successfully removed account ${removedAddress}`);
      navigate(Routes.wallet.path);
    }
  }, [activeAccount, lockAccounts, navigate, removeAccount, title]);

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

  const settingsListItemContent = useMemo(() => (
    <VStack width="100%">
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
        <Flex
          color={textColorDict[colorMode]}
          fontWeight={600}
          fontSize="md"
        >
          {renderTitle}
        </Flex>
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
    </VStack>
  ), [
    DividerComponent,
    colorMode,
    gridOnClick,
    hoverBgColorDict,
    iconAfter,
    iconBefore,
    renderTitle,
    templateColumns,
    textColorDict,
  ]);

  const settingsListItemContentWithRedirects = useMemo(() => {
    if (externalLink) {
      return (
        <VStack
          as="a"
          width="100%"
          alignItems="flex-start"
          href={externalLink}
          target="_blank"
          rel="noreferrer"
        >
          {settingsListItemContent}
        </VStack>
      );
    }
    if (path) {
      return (
        <ChakraLink width="100%" to={path}>
          { settingsListItemContent }
        </ChakraLink>
      );
    }
    if (title === 'View on Explorer') {
      const explorerAddress = activeAccount?.address
        ? `https://explorer.devnet.aptos.dev/account/${activeAccount.address}`
        : 'https://explorer.devnet.aptos.dev';
      return (
        <VStack
          as="a"
          width="100%"
          alignItems="flex-start"
          href={explorerAddress}
          target="_blank"
          rel="noreferrer"
        >
          {settingsListItemContent}
        </VStack>
      );
    }

    return settingsListItemContent;
  }, [activeAccount.address, externalLink, path, settingsListItemContent, title]);

  return (
    <VStack width="100%" alignItems="flex-start">
      {settingsListItemContentWithRedirects}
    </VStack>
  );
}
