// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  VStack,
  Button,
  useColorMode,
} from '@chakra-ui/react';
import { AddIcon } from '@chakra-ui/icons';
import React, { useMemo } from 'react';
import { useUnlockedAccounts } from 'core/hooks/useAccounts';
import { Routes } from 'core/routes';
import { Account } from 'shared/types';
import {
  switchAccountErrorToast,
  switchAccountToast,
} from 'core/components/Toast';
import { secondaryHeaderInputBgColor } from 'core/colors';
import { useNavigate } from 'react-router-dom';
import AccountView from './AccountView';

export const boxShadow = 'rgba(149, 157, 165, 0.2) 0px 0px 8px 4px';

export default function SwitchAccountBody() {
  const {
    accounts,
    switchAccount,
  } = useUnlockedAccounts();
  const { colorMode } = useColorMode();
  const navigate = useNavigate();

  const onSwitchAccount = (address: string) => {
    try {
      switchAccount(address);
      switchAccountToast(address);
      navigate(Routes.wallet.path);
    } catch {
      switchAccountErrorToast();
    }
  };

  const accountsList = useMemo(() => Object.values(accounts), [accounts]);

  const handleClickAddAccount = () => {
    navigate(Routes.addAccount.path);
  };

  return (
    <VStack mt={2} spacing={2} alignItems="left" display="flex" height="100%">
      <VStack gap={1} p={2} flex={1} overflow="scroll">
        {
        accountsList.map((account: Account) => (
          <AccountView
            account={account}
            showCheck
            boxShadow={boxShadow}
            onClick={onSwitchAccount}
            key={account.address}
            bgColor={{
              dark: 'gray.700',
              light: 'white',
            }}
          />
        ))
      }
      </VStack>
      <Button
        size="lg"
        width="100%"
        onClick={handleClickAddAccount}
        bgColor={secondaryHeaderInputBgColor[colorMode]}
        leftIcon={<AddIcon fontSize="xs" />}
      >
        Add Account
      </Button>
    </VStack>
  );
}
