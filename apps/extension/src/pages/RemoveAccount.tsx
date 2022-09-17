// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import React from 'react';

import {
  Button,
  Flex,
  Text,
  useColorMode,
  VStack,
} from '@chakra-ui/react';
import { useNavigate } from 'react-router-dom';
import WalletLayout from 'core/layouts/WalletLayout';
import { useActiveAccount, useUnlockedAccounts } from 'core/hooks/useAccounts';
import { collapseHexString } from 'core/utils/hex';
import { Routes } from 'core/routes';
import { removeButtonBg } from 'core/colors';
import { useAnalytics } from 'core/hooks/useAnalytics';
import { removeAccountEvents } from 'core/utils/analytics/events';
import { removeAccountErrorToast, removeAccountToast } from '../core/components/Toast';

function WarningIcon() {
  return (
    <svg width="57" height="57" viewBox="0 0 57 57" fill="none" xmlns="http://www.w3.org/2000/svg">
      <circle opacity="0.1" cx="28.5" cy="28.5" r="28.5" fill="#D76D61" />
      <path fillRule="evenodd" clipRule="evenodd" d="M10.5 29C10.5 19.34 18.34 11.5 28 11.5C37.66 11.5 45.5 19.34 45.5 29C45.5 38.66 37.66 46.5 28 46.5C18.34 46.5 10.5 38.66 10.5 29ZM19.25 29C19.25 29.9625 20.0375 30.75 21 30.75H35C35.9625 30.75 36.75 29.9625 36.75 29C36.75 28.0375 35.9625 27.25 35 27.25H21C20.0375 27.25 19.25 28.0375 19.25 29Z" fill="#D76D61" />
    </svg>
  );
}

const buttonBorderColor = {
  dark: 'gray.700',
  light: 'gray.200',
};

export default function RemoveAccount() {
  const activeAccount = useActiveAccount();
  const { colorMode } = useColorMode();
  const navigate = useNavigate();
  const { removeAccount } = useUnlockedAccounts();
  const { trackEvent } = useAnalytics();

  const handleRemove = async (e: React.MouseEvent<HTMLButtonElement>) => {
    e.preventDefault();

    try {
      await removeAccount(activeAccount.activeAccountAddress);
      const removedAddress = `${activeAccount.activeAccountAddress.slice(0, 4)}...${activeAccount.activeAccountAddress.slice(62)}`;
      removeAccountToast(`Successfully removed account ${removedAddress}`);
      trackEvent({ eventType: removeAccountEvents.REMOVE_ACCOUNT });
      navigate(Routes.wallet.path);
    } catch (err) {
      removeAccountErrorToast();
      trackEvent({
        eventType: removeAccountEvents.ERROR_REMOVE_ACCOUNT,
        params: { error: String(err) },
      });
    }
  };

  const handleCancel = (e: React.MouseEvent<HTMLButtonElement>) => {
    e.preventDefault();

    navigate(Routes.settings.path);
  };

  return (
    <WalletLayout title="Remove Account" showBackButton>
      <VStack paddingTop={4} pb={4} spacing={2} alignItems="center" height="100%">
        <VStack px={16} flex={1} justifyContent="center" minHeight="320px">
          <WarningIcon />
          <Text fontWeight={700} fontSize={18}>
            Remove
            {' '}
            {collapseHexString(activeAccount.activeAccountAddress)}
            {'  '}
            ?
          </Text>
          <Text textAlign="center" fontSize={17}>
            Although you are removing this from your Aptos wallet,
            you&apos;ll be able to retrieve if using your mnemonic phrase.
          </Text>
        </VStack>
        <Flex width="375px" justify="flex-end" alignItems="center" px={4} py={4} borderTop="1px" borderColor={buttonBorderColor[colorMode]}>
          <VStack width="100%">
            <Button width="100%" bgColor={removeButtonBg[colorMode]} color="white" onClick={handleRemove}>Remove</Button>
            <Button width="100%" onClick={handleCancel}>Cancel</Button>
          </VStack>
        </Flex>
      </VStack>
    </WalletLayout>
  );
}
