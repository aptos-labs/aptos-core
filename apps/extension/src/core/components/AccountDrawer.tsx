// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import React from 'react';
import {
  Button,
  Drawer,
  DrawerBody,
  DrawerContent, DrawerFooter,
  DrawerHeader,
  DrawerOverlay,
  Grid, ModalProps,
  Text, useColorMode, useRadioGroup, VStack,
} from '@chakra-ui/react';
import { AddIcon } from '@chakra-ui/icons';
import { secondaryBorderColor } from 'core/colors';
import { useActiveAccount, useUnlockedAccounts } from 'core/hooks/useAccounts';
import AccountDrawerItem from 'core/components/AccountDrawerItem';
import ChakraLink from 'core/components/ChakraLink';
import Routes from 'core/routes';
import { useNavigate } from 'react-router-dom';
import {
  removeAccountErrorToast,
  removeAccountToast,
  switchAccountErrorToast,
  switchAccountToast,
} from 'core/components/Toast';

export type AccountDrawerProps = Omit<ModalProps, 'children'>;

export default function AccountDrawer({ isOpen, onClose }: AccountDrawerProps) {
  const { colorMode } = useColorMode();
  const navigate = useNavigate();

  const {
    accounts,
    removeAccount,
    switchAccount,
  } = useUnlockedAccounts();
  const { activeAccountAddress } = useActiveAccount();

  const onSwitchAccount = (address: string) => {
    try {
      switchAccount(address);
      switchAccountToast(address);
      onClose();
    } catch {
      switchAccountErrorToast();
    }
  };

  const { getRadioProps, getRootProps, setValue } = useRadioGroup({
    defaultValue: activeAccountAddress,
    onChange: onSwitchAccount,
  });

  const onRemoveAccount = async (address: string) => {
    try {
      await removeAccount(address);
      const firstAvailableAddress = Object.keys(accounts!).filter((a) => a !== address)[0];

      let toastMessage;
      if (!firstAvailableAddress) {
        toastMessage = 'No other accounts in wallet, signing out';
      } else if (address === activeAccountAddress) {
        toastMessage = `Switching to account with address: ${firstAvailableAddress.substring(0, 6)}...`;
        setValue(firstAvailableAddress);
      } else {
        toastMessage = `Using the same account with address: ${activeAccountAddress.substring(0, 6)}...`;
      }
      removeAccountToast(toastMessage);

      onClose();
      navigate(Routes.wallet.path);
    } catch {
      removeAccountErrorToast();
    }
  };

  return (
    <Drawer placement="bottom" onClose={onClose} isOpen={isOpen}>
      <DrawerOverlay />
      <DrawerContent>
        <DrawerHeader px={4} borderBottomWidth="1px">
          <Grid templateColumns="1fr 136px">
            <Text>Accounts</Text>
            <ChakraLink
              to={Routes.addAccount.path}
              display="flex"
              justifyContent="flex-end"
            >
              <Button
                colorScheme="teal"
                size="sm"
                leftIcon={<AddIcon />}
              >
                New account
              </Button>
            </ChakraLink>
          </Grid>
        </DrawerHeader>
        <DrawerBody px={4} maxH="400px">
          { accounts && activeAccountAddress ? (
            <VStack spacing={2} width="100%" py={2} {...getRootProps()}>
              {
              Object.keys(accounts).map((address) => (
                <AccountDrawerItem
                  key={address}
                  account={accounts[address]}
                  onRemove={onRemoveAccount}
                  {...getRadioProps({ value: address })}
                />
              ))
            }
            </VStack>
          ) : null }
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
  );
}
