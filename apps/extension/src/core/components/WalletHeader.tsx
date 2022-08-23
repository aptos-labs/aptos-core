// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import React, { MouseEventHandler } from 'react';
import {
  Box,
  Center,
  Grid,
  HStack,
  Input,
  InputGroup,
  InputLeftAddon,
  InputRightAddon,
  Tooltip,
  Text,
  useClipboard,
  useColorMode,
  useDisclosure,
} from '@chakra-ui/react';
import {
  ChevronLeftIcon, DragHandleIcon,
} from '@chakra-ui/icons';
import {
  secondaryBorderColor,
  secondaryHeaderInputBgColor,
  secondaryHeaderInputHoverBgColor,
} from 'core/colors';
import { IoIosWallet } from '@react-icons/all-files/io/IoIosWallet';
import { useNavigate } from 'react-router-dom';
import AccountDrawer from 'core/components/AccountDrawer';
import useGlobalStateContext from 'core/hooks/useGlobalState';

interface ButtonProps {
  onClick: MouseEventHandler<HTMLDivElement>;
}

function AccountCircle({ onClick }: ButtonProps) {
  return (
    <Box
      height="40px"
      width="40px"
      background="gray"
      borderRadius="2rem"
      cursor="pointer"
      onClick={onClick}
    />
  );
}

function BackButton({ onClick }: ButtonProps) {
  return (
    <Box
      height="44px"
      width="44px"
      background="#F2F4F8"
      borderRadius="0.5rem"
      cursor="pointer"
      onClick={onClick}
    >
      <ChevronLeftIcon color="#333333" width="100%" height="100%" />
    </Box>
  );
}

interface NavigationBarProps {
  accessoryButton?: React.ReactNode,
  showBackButton?: boolean;
  title?: string
}

function NavigationBar({
  accessoryButton,
  showBackButton,
  title,
}: NavigationBarProps) {
  const navigate = useNavigate();
  const { colorMode } = useColorMode();
  const { isOpen, onClose, onOpen } = useDisclosure();

  return (
    <Box>
      <HStack
        maxW="100%"
        width="100%"
        py={4}
        height="84px"
        borderBottomColor={secondaryBorderColor[colorMode]}
        borderBottomWidth="1px"
        justifyContent="space-between"
        padding={4}
      >
        <HStack>
          {(showBackButton)
            ? (
              <BackButton onClick={() => navigate(-1)} />
            )
            : null}
          <Text fontSize="xl" fontWeight="semibold">
            {title}
          </Text>
        </HStack>
        <HStack spacing={4}>
          {accessoryButton}
          <Tooltip label="Switch wallet" closeDelay={300}>
            <AccountCircle onClick={onOpen} />
          </Tooltip>
        </HStack>

      </HStack>
      <AccountDrawer isOpen={isOpen} onClose={onClose} />
    </Box>

  );
}

export default function WalletHeader({
  accessoryButton,
  showBackButton,
  title,
}: NavigationBarProps) {
  const { activeAccount, activeAccountAddress } = useGlobalStateContext();
  const { isOpen, onClose, onOpen } = useDisclosure();
  const { colorMode } = useColorMode();
  const { hasCopied, onCopy } = useClipboard(activeAccountAddress || '');
  const navigate = useNavigate();

  if ((!process.env.NODE_ENV || process.env.NODE_ENV === 'development')) {
    return (
      <NavigationBar
        accessoryButton={accessoryButton}
        title={title}
        showBackButton={showBackButton}
      />
    );
  }

  return (
    <Grid
      maxW="100%"
      width="100%"
      py={4}
      height="64px"
      templateColumns={showBackButton ? '40px 1fr' : '1fr'}
      borderBottomColor={secondaryBorderColor[colorMode]}
      borderBottomWidth="1px"
    >
      {(showBackButton) ? (
        <Center cursor="pointer" onClick={() => navigate(-1)}>
          <ChevronLeftIcon fontSize="xl" />
        </Center>
      ) : <Box />}
      <Center width="100%">
        <HStack
          px={4}
          pl={showBackButton ? 0 : 4}
          width="100%"
        >
          <InputGroup size="sm">
            <InputLeftAddon
              borderLeftRadius=".5rem"
              bgColor={secondaryHeaderInputBgColor[colorMode]}
              borderColor={secondaryBorderColor[colorMode]}
              borderWidth="0px"
            >
              <IoIosWallet />
            </InputLeftAddon>
            <Tooltip
              label={hasCopied ? 'Copied!' : 'Copy address'}
              closeDelay={300}
            >
              <Input
                size="sm"
                readOnly
                value={`${activeAccount?.name} (${activeAccountAddress})`}
                onClick={onCopy}
                borderColor={secondaryBorderColor[colorMode]}
                bgColor={secondaryHeaderInputBgColor[colorMode]}
                borderWidth="0px"
                borderRadius={0}
                cursor="pointer"
                textOverflow="ellipsis"
                _hover={{
                  backgroundColor: secondaryHeaderInputHoverBgColor[colorMode],
                }}
              />
            </Tooltip>
            <Tooltip label="Switch wallet" closeDelay={300}>
              <InputRightAddon
                onClick={onOpen}
                borderRightRadius=".5rem"
                borderColor={secondaryBorderColor[colorMode]}
                bgColor={secondaryHeaderInputBgColor[colorMode]}
                borderWidth="0px"
                cursor="pointer"
                _hover={{
                  backgroundColor: secondaryHeaderInputHoverBgColor[colorMode],
                }}
              >
                <DragHandleIcon />
              </InputRightAddon>
            </Tooltip>
          </InputGroup>
          <AccountDrawer isOpen={isOpen} onClose={onClose} />
        </HStack>
      </Center>
      <Box />
    </Grid>
  );
}
