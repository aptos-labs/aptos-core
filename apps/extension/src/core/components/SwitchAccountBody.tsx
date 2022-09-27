// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  VStack,
  Button,
  useColorMode,
  Box,
} from '@chakra-ui/react';
import { AddIcon } from '@chakra-ui/icons';
import React, { useMemo } from 'react';
import { useUnlockedAccounts } from 'core/hooks/useAccounts';
import { Routes } from 'core/routes';
import { Account } from 'shared/types';
import { secondaryHeaderInputBgColor } from 'core/colors';
import { useNavigate } from 'react-router-dom';
import AccountView from './AccountView';

export const boxShadow = 'rgba(0, 0, 0, 0.05) 0px 4px 24px 0px';

const bgColor = {
  dark: 'gray.700',
  light: 'white',
};

export default function SwitchAccountBody() {
  const {
    accounts,
  } = useUnlockedAccounts();
  const { colorMode } = useColorMode();
  const navigate = useNavigate();

  const accountsList: Account[] = useMemo(() => Object.values(accounts), [accounts]);

  const handleClickAddAccount = () => {
    navigate(Routes.addAccount.path);
  };

  return (
    <Box width="100%" height="100%" position="relative">
      <Box background="navy.900" height="15%" position="absolute" width="100%" />
      <VStack position="absolute" pt={4} alignItems="stretch" height="100%" width="100%">
        <VStack gap={1} flex={1}>
          {
          accountsList.map((account: Account) => (
            <Box px={4} width="100%" key={account.address}>
              <AccountView
                account={account}
                bgColor={bgColor}
              />
            </Box>
          ))
        }
        </VStack>
        <Box px={4} width="100%" minHeight="58px">
          <Button
            size="lg"
            width="100%"
            onClick={handleClickAddAccount}
            bgColor={secondaryHeaderInputBgColor[colorMode]}
            leftIcon={<AddIcon fontSize="xs" />}
          >
            Add Account
          </Button>
        </Box>
      </VStack>
    </Box>
  );
}
