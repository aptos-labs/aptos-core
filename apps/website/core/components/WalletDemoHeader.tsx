// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import React from 'react';
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
  useColorMode,
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

interface WalletHeaderProps {
  showBackButton?: boolean;
}

export default function WalletDemoHeader({
  showBackButton,
}: WalletHeaderProps) {
  const { colorMode } = useColorMode();

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
        <Center cursor="pointer">
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
            <Input
              size="sm"
              readOnly
              value="0xAlice"
              borderColor={secondaryBorderColor[colorMode]}
              bgColor={secondaryHeaderInputBgColor[colorMode]}
              borderWidth="0px"
              borderRadius={0}
              cursor="pointer"
              textOverflow="ellipsis"
              _hover={{
                backgroundColor: secondaryHeaderInputHoverBgColor[colorMode],
              }}
              isDisabled
            />
            <Tooltip label="Switch wallet" closeDelay={300}>
              <InputRightAddon
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
        </HStack>
      </Center>
      <Box />
    </Grid>
  );
}
