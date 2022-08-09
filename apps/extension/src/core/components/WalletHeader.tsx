// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  Box,
  Button,
  Center,
  Drawer,
  DrawerBody,
  DrawerContent,
  DrawerFooter,
  DrawerHeader,
  DrawerOverlay,
  Grid,
  HStack,
  Input,
  InputGroup,
  InputLeftAddon,
  InputRightAddon,
  Text,
  Tooltip,
  useClipboard,
  useColorMode,
  useDisclosure,
} from '@chakra-ui/react';
import React, { useState } from 'react';
import { useWalletState } from 'core/hooks/useWalletState';
import {
  AddIcon, ChevronLeftIcon, DragHandleIcon,
} from '@chakra-ui/icons';
import {
  secondaryBorderColor,
  secondaryHeaderInputBgColor,
  secondaryHeaderInputHoverBgColor,
} from 'core/colors';
import { IoIosWallet } from '@react-icons/all-files/io/IoIosWallet';
import { useNavigate } from 'react-router-dom';
import Routes from 'core/routes';
import WalletDrawerBody from './WalletDrawerBody';
import ChakraLink from './ChakraLink';

interface WalletHeaderProps {
  showBackButton?: boolean;
}

export default function WalletHeader({
  showBackButton,
}: WalletHeaderProps) {
  const { aptosAccount } = useWalletState();
  const [isLoading] = useState<boolean>(false);
  const navigate = useNavigate();
  const { isOpen, onClose, onOpen } = useDisclosure();
  const { colorMode } = useColorMode();
  const { hasCopied, onCopy } = useClipboard(
    aptosAccount?.address().hex() || '',
  );

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
                value={aptosAccount?.address().hex()}
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
          <Drawer placement="bottom" onClose={onClose} isOpen={isOpen}>
            <DrawerOverlay />
            <DrawerContent>
              <DrawerHeader
                px={4}
                borderBottomWidth="1px"
              >
                <Grid templateColumns="1fr 136px">
                  <Text>Accounts</Text>
                  <ChakraLink
                    to={Routes.addAccount.routePath}
                    display="flex"
                    justifyContent="flex-end"
                  >
                    <Button
                      colorScheme="teal"
                      size="sm"
                      leftIcon={<AddIcon />}
                      isLoading={isLoading}
                    >
                      New Account
                    </Button>
                  </ChakraLink>
                </Grid>
              </DrawerHeader>
              <DrawerBody px={4} maxH="400px">
                <WalletDrawerBody />
              </DrawerBody>
              <DrawerFooter
                px={4}
                borderTopWidth="1px"
                borderTopColor={secondaryBorderColor[colorMode]}
              >
                <Button onClick={onClose}>
                  Close
                </Button>
              </DrawerFooter>
            </DrawerContent>
          </Drawer>
        </HStack>
      </Center>
      <Box />
    </Grid>
  );
}
