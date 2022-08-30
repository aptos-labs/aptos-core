// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import React, { LegacyRef, useMemo, useState } from 'react';
import {
  Box,
  Center,
  Grid,
  Text,
  useColorMode,
  VStack,
  Button,
  HStack,
  Tooltip,
} from '@chakra-ui/react';
import { useNavigate } from 'react-router-dom';
import { Routes } from 'core/routes';

import { RiFileCopyLine } from '@react-icons/all-files/ri/RiFileCopyLine';
import { HiPencil } from '@react-icons/all-files/hi/HiPencil';
import { AiFillCheckCircle } from '@react-icons/all-files/ai/AiFillCheckCircle';
import {
  Account,
} from 'shared/types';
import {
  secondaryGridBgColor,
  textColor,
  accountViewBgColor,
  secondaryTextColor,
  checkCircleSuccessBg,
} from 'core/colors';
import AccountCircle from 'core/components/AccountCircle';
import { useActiveAccount } from 'core/hooks/useAccounts';
import { collapseHexString } from 'core/utils/hex';
import Copyable from './Copyable';

interface AccountViewProps {
  account?: Account
  allowEdit?: boolean;
  bgColor?: any;
  boxShadow?: string;
  hoverBgColor?: any;
  onClick?: (address: string) => void;
  showCheck?: boolean;
}

const AccountView = React.forwardRef(({
  account,
  boxShadow = '',
  allowEdit = false,
  showCheck = false,
  onClick,
  bgColor = secondaryGridBgColor,
  hoverBgColor = accountViewBgColor,
}: AccountViewProps, ref: LegacyRef<HTMLImageElement>) => {
  const { colorMode } = useColorMode();
  const navigate = useNavigate();
  const { activeAccount } = useActiveAccount();
  const [opacity, setOpacity] = useState(0);

  const displayAccount = useMemo(() => account ?? activeAccount, [account, activeAccount]);

  const { hasCopied, onCopy } = useClipboard(displayAccount?.address || '');

    return displayActiveAccount?.address;
  }, [account, activeAccount]);

  const handleClickEditAccount = (e: React.MouseEvent<HTMLButtonElement>) => {
    e.preventDefault();
    navigate(Routes.rename_account.path);
  };

  return (
    <Grid
      ref={ref}
      onClick={handleClickAccount}
      templateColumns="48px 1fr 32px"
      p={4}
      width="100%"
      cursor="pointer"
      gap={2}
      borderWidth={1}
      borderColor="gray.100"
      boxShadow={boxShadow}
      bgColor={bgColor[colorMode]}
      borderRadius=".5rem"
      _hover={{
        bgColor: hoverBgColor[colorMode],
      }}
    >
      <Center width="100%">
        <AccountCircle account={displayAccount} size={40} />
      </Center>
      <VStack width="100%" alignItems="flex-start" spacing={0}>
        <Text color={textColor[colorMode]} fontWeight={600} fontSize="md">
          {displayAccount.name}
        </Text>
        <Copyable value={displayActiveAccountAddress}>
          <HStack alignItems="baseline">
            <Text fontSize="sm" color={secondaryTextColor[colorMode]}>
              {collapseHexString(displayActiveAccountAddress)}
            </Text>
            <RiFileCopyLine />
          </HStack>
        </Copyable>
      </VStack>
      <Tooltip label="Rename">
        <Button
          borderRadius="100%"
          colorScheme="teal"
          variant="ghost"
          bg="none"
          p={0}
          onClick={handleClickEditAccount}
        >
          <HiPencil size={20} />
        </Button>
      </Tooltip>
    </Grid>
  );
});

export default AccountView;
