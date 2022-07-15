// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  Box,
  Button,
  ButtonGroup,
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
import useWalletState from 'core/hooks/useWalletState';
import {
  AddIcon, ChevronLeftIcon, DragHandleIcon,
} from '@chakra-ui/icons';
import {
  secondaryBorderColor,
  secondaryHeaderInputBgColor,
  secondaryHeaderInputHoverBgColor,
} from 'core/colors';
import { IoIosWallet } from '@react-icons/all-files/io/IoIosWallet';
import { AptosAccount } from 'aptos';
import { useNavigate } from 'react-router-dom';
import Routes from 'core/routes';
import ChakraLink from './ChakraLink';
import WalletDrawerBody from './WalletDrawerBody';

interface WalletHeaderProps {
  backPage?: string;
}

export default function WalletHeader({
  backPage,
}: WalletHeaderProps) {
  const { addAccount, aptosAccount } = useWalletState();
  const [isLoading, setIsLoading] = useState<boolean>(false);
  const navigate = useNavigate();
  const { isOpen, onClose, onOpen } = useDisclosure();
  const { colorMode } = useColorMode();
  const { hasCopied, onCopy } = useClipboard(
    aptosAccount?.address().hex() || '',
  );

  const newWalletOnClick = async () => {
    const blankAptosAccount = new AptosAccount();
    setIsLoading(true);
    await addAccount({ account: blankAptosAccount });
    setIsLoading(false);
    navigate(Routes.login.routePath);
  };

  return (
    <Grid
      maxW="100%"
      width="100%"
      py={4}
      height="64px"
      templateColumns={backPage ? '40px 1fr' : '1fr'}
      borderBottomColor={secondaryBorderColor[colorMode]}
      borderBottomWidth="1px"
    >
      {(backPage) ? (
        <Center>
          <ChakraLink to={backPage}>
            <ChevronLeftIcon fontSize="xl" aria-label={backPage} />
          </ChakraLink>
        </Center>
      ) : <Box />}
      <Center width="100%">
        <HStack
          px={4}
          pl={(backPage) ? 0 : 4}
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
                <Grid templateColumns="1fr 150px">
                  <Text>Wallets</Text>
                  <ButtonGroup justifyContent="flex-end">
                    <Button
                      colorScheme="teal"
                      size="sm"
                      leftIcon={<AddIcon />}
                      onClick={newWalletOnClick}
                      isLoading={isLoading}
                    >
                      New Wallet
                    </Button>
                  </ButtonGroup>
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
