// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import React, { LegacyRef, useMemo } from 'react';
import {
  Center,
  Grid,
  Text,
  useColorMode,
  VStack,
  HStack,
  Button,
  Tooltip,
} from '@chakra-ui/react';
import { useNavigate } from 'react-router-dom';
import { Routes } from 'core/routes';
import { RiFileCopyLine } from '@react-icons/all-files/ri/RiFileCopyLine';
import { AiFillCheckCircle } from '@react-icons/all-files/ai/AiFillCheckCircle';
import { HiPencil } from '@react-icons/all-files/hi/HiPencil';
import {
  Account,
} from 'shared/types';
import {
  secondaryGridBgColor,
  textColor,
  accountViewBgColor,
  secondaryTextColor,
  secondaryBorderColor,
  checkCircleSuccessBg,
} from 'core/colors';
import AccountCircle from 'core/components/AccountCircle';
import { useActiveAccount, useUnlockedAccounts } from 'core/hooks/useAccounts';
import { collapseHexString } from 'core/utils/hex';
import {
  switchAccountErrorToast,
  switchAccountToast,
} from 'core/components/Toast';
import Copyable from './Copyable';

interface AccountViewProps {
  account?: Account
  allowEdit?: boolean;
  bgColor?: any;
  hoverBgColor?: any;
}

const AccountView = React.forwardRef(({
  account,
  bgColor = secondaryGridBgColor,
  hoverBgColor = accountViewBgColor,
  allowEdit = false,
}: AccountViewProps, ref: LegacyRef<HTMLImageElement>) => {
  const { colorMode } = useColorMode();
  const navigate = useNavigate();
  const { activeAccount } = useActiveAccount();
  const {
    switchAccount,
  } = useUnlockedAccounts();

  const switchAccountOnClick = () => {
    try {
      if (account?.address) {
        switchAccount(account?.address);
        switchAccountToast(account?.address);
        navigate(Routes.wallet.path);
      }
    } catch {
      switchAccountErrorToast();
    }
  };

  const editAccountOnClick = (e: React.MouseEvent<HTMLButtonElement>) => {
    e.preventDefault();
    navigate(Routes.rename_account.path);
  };

  const displayAccount = useMemo(() => account ?? activeAccount, [account, activeAccount]);

  const doNotAllowEditAccountButton = (activeAccount.address === displayAccount.address)
    ? <AiFillCheckCircle size={32} color={checkCircleSuccessBg[colorMode]} /> : null;

  const allowEditAccountButton = (
    <Tooltip label="Rename">
      <Button
        borderRadius="100%"
        colorScheme="teal"
        variant="ghost"
        bg="none"
        p={0}
        onClick={editAccountOnClick}
      >
        <HiPencil size={20} />
      </Button>
    </Tooltip>
  );

  return (
    <Grid
      ref={ref}
      templateColumns="48px 1fr 32px"
      p={4}
      width="100%"
      cursor="pointer"
      gap={2}
      borderWidth={1}
      borderColor={secondaryBorderColor[colorMode]}
      bgColor={bgColor[colorMode]}
      borderRadius=".5rem"
      _hover={{
        bgColor: hoverBgColor[colorMode],
      }}
      onClick={switchAccountOnClick}
    >
      <Center width="100%">
        <AccountCircle account={displayAccount} size={40} />
      </Center>
      <VStack width="100%" alignItems="flex-start" spacing={0}>
        <Text color={textColor[colorMode]} fontWeight={600} fontSize="md">
          {displayAccount.name || 'Account'}
        </Text>
        <Copyable value={displayAccount.address}>
          <HStack alignItems="baseline">
            <Text fontSize="sm" color={secondaryTextColor[colorMode]}>
              {collapseHexString(displayAccount.address)}
            </Text>
            <RiFileCopyLine />
          </HStack>
        </Copyable>
      </VStack>
      {
        (allowEdit) ? (
          allowEditAccountButton
        ) : (
          doNotAllowEditAccountButton
        )
      }
    </Grid>
  );
});

export default AccountView;
