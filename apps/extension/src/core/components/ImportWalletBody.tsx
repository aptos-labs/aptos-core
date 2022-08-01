// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { Button, VStack } from '@chakra-ui/react';
import { FaKey } from '@react-icons/all-files/fa/FaKey';
import { BsLayoutTextSidebar } from '@react-icons/all-files/bs/BsLayoutTextSidebar';
import React from 'react';
import ChakraLink from './ChakraLink';

export default function ImportWalletBody() {
  return (
    <VStack px={4} spacing={4} width="100%" pt={4}>
      <ChakraLink to="/import/private-key" width="100%">
        <Button width="100%" height={16} leftIcon={<FaKey />}>
          Import private key
        </Button>
      </ChakraLink>
      <ChakraLink to="/import/mnemonic" width="100%">
        <Button width="100%" height={16} leftIcon={<BsLayoutTextSidebar />}>
          Import mnemonic
        </Button>
      </ChakraLink>
    </VStack>
  );
}
