// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { Button, VStack } from '@chakra-ui/react';
import { FaKey } from '@react-icons/all-files/fa/FaKey';
import { BsLayoutTextSidebar } from '@react-icons/all-files/bs/BsLayoutTextSidebar';
import React from 'react';
import Routes from 'core/routes';
import { PlusSquareIcon } from '@chakra-ui/icons';
import ChakraLink from './ChakraLink';

export default function AddAccountBody() {
  return (
    <VStack px={4} spacing={4} width="100%" pt={4}>
      <ChakraLink to={Routes.createAccount.routePath} width="100%">
        <Button
          width="100%"
          height={16}
          leftIcon={<PlusSquareIcon />}
          justifyContent="flex-start"
        >
          Create new account
        </Button>
      </ChakraLink>
      <ChakraLink to={Routes.importWalletPrivateKey.routePath} width="100%">
        <Button
          width="100%"
          height={16}
          leftIcon={<FaKey />}
          justifyContent="flex-start"
        >
          Import private key
        </Button>
      </ChakraLink>
      <ChakraLink to={Routes.importWalletMnemonic.routePath} width="100%">
        <Button
          width="100%"
          height={16}
          leftIcon={<BsLayoutTextSidebar />}
          justifyContent="flex-start"
        >
          Import mnemonic
        </Button>
      </ChakraLink>
    </VStack>
  );
}
