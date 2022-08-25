// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import React, { useMemo } from 'react';
import {
  Center,
  Grid, Text, useColorMode, VStack, Flex, Button,
  useClipboard, Tooltip,
} from '@chakra-ui/react';
import { useNavigate } from 'react-router-dom';
import { Routes } from 'core/routes';
import { RiFileCopyLine } from '@react-icons/all-files/ri/RiFileCopyLine';
import { HiPencil } from '@react-icons/all-files/hi/HiPencil';
import {
  AptosAccountState,
} from 'core/types/stateTypes';
import {
  secondaryGridBgColor,
  textColor,
  accountViewBgColor,
  secondaryTextColor,
} from 'core/colors';
import AccountCircle from 'core/components/AccountCircle';
import { useActiveAccount } from 'core/hooks/useAccounts';

type AccountViewProps = {
  account?: AptosAccountState
};

function AccountView({ account: accountFromProps }: AccountViewProps) {
  const { colorMode } = useColorMode();
  const navigate = useNavigate();
  const { activeAccount } = useActiveAccount();

  const displayActiveAccountAddress = useMemo(() => {
    const displayActiveAccount = accountFromProps || activeAccount;

    if (!displayActiveAccount) return '';
    if (typeof displayActiveAccount?.address === 'string') {
      return displayActiveAccount?.address;
    }

    return displayActiveAccount?.address().toString();
  }, [accountFromProps, activeAccount]);

  const { hasCopied, onCopy } = useClipboard(displayActiveAccountAddress || '');

  const handleClickEditAccount = (e: React.MouseEvent<HTMLButtonElement>) => {
    e.preventDefault();
    navigate(Routes.rename_account.path);
  };

  const beginAddress = useMemo(() => displayActiveAccountAddress?.slice(0, 4) || '', [displayActiveAccountAddress]);
  const endAddress = useMemo(() => displayActiveAccountAddress?.slice(62) || '', [displayActiveAccountAddress]);

  return (
    <Grid
      templateColumns="32px 1fr 32px"
      p={4}
      width="100%"
      cursor="pointer"
      gap={2}
      bgColor={secondaryGridBgColor[colorMode]}
      borderRadius=".5rem"
      _hover={{
        bgColor: accountViewBgColor[colorMode],
      }}
    >
      <Center width="100%">
        <AccountCircle />
      </Center>
      <VStack width="100%" alignItems="flex-start" spacing={0}>
        <Text color={textColor[colorMode]} fontWeight={600} fontSize="md">
          {activeAccount?.name}
        </Text>
        <Tooltip label={hasCopied ? 'Copied!' : 'Copy'} closeDelay={300}>
          <Text fontSize="sm" color={secondaryTextColor[colorMode]} onClick={onCopy}>
            <Flex flexDirection="row" gap={1} alignItems="baseline">
              {beginAddress}
              ...
              {endAddress}
              <RiFileCopyLine />
            </Flex>
          </Text>
        </Tooltip>
      </VStack>
      <Button bg="none" p={0} onClick={handleClickEditAccount}>
        <HiPencil size={20} />
      </Button>
    </Grid>
  );
}

export default AccountView;
