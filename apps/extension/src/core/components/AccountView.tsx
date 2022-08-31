// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import React, { LegacyRef, useMemo, useState } from 'react';
import {
  Box,
  Center,
  Grid, Text, useColorMode, VStack, Flex, Button,
  useClipboard, Tooltip,
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

  const handleClickEditAccount = (e: React.MouseEvent<HTMLButtonElement>) => {
    e.preventDefault();
    navigate(Routes.rename_account.path);
  };

  const handleClickAccount = (e: React.MouseEvent<HTMLDivElement>) => {
    if (onClick && opacity === 0) {
      e.preventDefault();
      onClick(displayAccount?.address);
    }
  };

  const beginAddress = useMemo(() => displayAccount.address?.slice(0, 6) || '', [displayAccount]);
  const endAddress = useMemo(() => displayAccount.address?.slice(62) || '', [displayAccount]);

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
        <Tooltip label={hasCopied ? 'Copied!' : 'Copy'} closeDelay={300}>
          <Text
            fontSize="sm"
            color={secondaryTextColor[colorMode]}
            onClick={onCopy}
            onMouseEnter={() => setOpacity(1)}
            onMouseLeave={() => setOpacity(0)}
          >
            <Flex flexDirection="row" gap={1} alignItems="baseline">
              {beginAddress}
              ...
              {endAddress}
              <Box opacity={opacity}>
                <RiFileCopyLine />
              </Box>
            </Flex>
          </Text>
        </Tooltip>
      </VStack>
      {(activeAccount.address === displayAccount.address && showCheck
        ? <AiFillCheckCircle size={32} color={checkCircleSuccessBg[colorMode]} /> : null)}
      {(allowEdit ? (
        <Button bg="none" p={0} onClick={handleClickEditAccount}>
          <HiPencil size={20} />
        </Button>
      ) : null)}
    </Grid>
  );
});

export default AccountView;
