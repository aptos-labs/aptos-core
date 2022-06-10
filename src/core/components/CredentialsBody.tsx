// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  VStack,
  Flex,
  SimpleGrid,
  Tag,
  useDisclosure,
  Heading,
  Drawer,
  DrawerOverlay,
  DrawerContent,
  DrawerHeader,
  DrawerBody,
} from '@chakra-ui/react';
import React from 'react';
import useWalletState from 'core/hooks/useWalletState';
import { CredentialHeaderAndBody } from 'pages/CreateWallet';
import { CredentialRow } from 'pages/Settings';

export default function CredentialsBody() {
  const { isOpen, onClose, onOpen } = useDisclosure();
  const { aptosAccount } = useWalletState();
  const privateKeyObject = aptosAccount?.toPrivateKeyObject();
  const privateKeyHex = privateKeyObject?.privateKeyHex;
  const publicKeyHex = privateKeyObject?.publicKeyHex;
  const address = privateKeyObject?.address;

  return (
    <>
      <SimpleGrid columns={2} width="100%" pb={4}>
        <Flex>
          <Heading fontSize="xl">Credentials</Heading>
        </Flex>
        <Flex justifyContent="right">
          <Tag size="sm" onClick={onOpen} cursor="pointer">
            View more
          </Tag>
          <Drawer
            isOpen={isOpen}
            onClose={onClose}
            placement="bottom"
          >
            <DrawerOverlay />
            <DrawerContent>
              <DrawerHeader borderBottomWidth="1px" px={4}>
                Credentials
              </DrawerHeader>
              <DrawerBody px={4}>
                <VStack mt={2} spacing={4} pb={8} alignItems="flex-start">
                  <CredentialHeaderAndBody
                    header="Private key"
                    body={privateKeyHex}
                  />
                  <CredentialHeaderAndBody
                    header="Public key"
                    body={publicKeyHex}
                  />
                  <CredentialHeaderAndBody
                    header="Address"
                    body={address}
                  />
                </VStack>
              </DrawerBody>
            </DrawerContent>
          </Drawer>
        </Flex>
      </SimpleGrid>
      <VStack mt={2} spacing={2} alignItems="left">
        <CredentialRow
          header="Private key"
          body={privateKeyHex}
        />
        <CredentialRow
          header="Public key"
          body={publicKeyHex}
        />
        <CredentialRow
          header="Address"
          body={address}
        />
      </VStack>
    </>
  );
}
