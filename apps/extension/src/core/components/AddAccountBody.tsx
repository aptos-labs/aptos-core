// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { Button, VStack, Text } from '@chakra-ui/react';
import { FaKey } from '@react-icons/all-files/fa/FaKey';
import { BsLayoutTextSidebar } from '@react-icons/all-files/bs/BsLayoutTextSidebar';
import React from 'react';
import Routes from 'core/routes';
import { PlusSquareIcon } from '@chakra-ui/icons';
import { CreateWalletViaImportFormValues } from 'core/layouts/CreateWalletViaImportLayout';
import { useFormContext } from 'react-hook-form';
import ChakraLink from './ChakraLink';

interface AddAccountBodyProps {
  px?: number;
}

export default function AddAccountBody({
  px = 4,
}: AddAccountBodyProps) {
  return (
    <VStack px={px} spacing={4} width="100%" pt={4}>
      <ChakraLink to={Routes.createAccount.path} width="100%">
        <Button
          width="100%"
          height={16}
          leftIcon={<PlusSquareIcon />}
          justifyContent="flex-start"
        >
          Create new account
        </Button>
      </ChakraLink>
      <ChakraLink to={Routes.importWalletPrivateKey.path} width="100%">
        <Button
          width="100%"
          height={16}
          leftIcon={<FaKey />}
          justifyContent="flex-start"
        >
          Import private key
        </Button>
      </ChakraLink>
      <ChakraLink to={Routes.importWalletMnemonic.path} width="100%">
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

export function NoWalletAddAccountBody({
  px = 4,
}: AddAccountBodyProps) {
  const {
    setValue,
    watch,
  } = useFormContext<CreateWalletViaImportFormValues>();

  const importType = watch('importType');

  const isImportTypeMnemonic = importType === 'mnemonic';
  const isImportTypePrivateKey = importType === 'privateKey';

  const importPrivateKeyOnClick = () => {
    setValue('importType', 'privateKey');
  };

  const importMnemonicOnClick = () => {
    setValue('importType', 'mnemonic');
  };

  return (
    <VStack px={px} spacing={4} width="100%" pt={4} alignContent="center" height="100%" justifyContent="center">
      <Button
        width="100%"
        height={20}
        leftIcon={<FaKey size={20} />}
        justifyContent="flex-start"
        onClick={importPrivateKeyOnClick}
        colorScheme={isImportTypePrivateKey ? 'salmon' : undefined}
      >
        <Text fontSize={18}>Import private key</Text>
      </Button>
      <Button
        width="100%"
        height={20}
        leftIcon={<BsLayoutTextSidebar size={20} />}
        justifyContent="flex-start"
        onClick={importMnemonicOnClick}
        colorScheme={isImportTypeMnemonic ? 'salmon' : undefined}
      >
        <Text fontSize={18}>Import mnemonic</Text>
      </Button>
    </VStack>
  );
}
