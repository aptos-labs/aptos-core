// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  Heading,
  Input,
  InputGroup,
  InputLeftElement,
  SimpleGrid,
  Text,
  useColorMode,
  VStack,
  HStack,
} from '@chakra-ui/react';
import { secondaryHeaderInputBgColor } from 'core/colors';
import { type CreateWalletFormValues } from 'core/layouts/CreateWalletLayout';
import React, { useEffect } from 'react';
import { useFormContext } from 'react-hook-form';

const borderColor = {
  dark: 'gray.700',
  light: 'white',
};

type MnemonicValueType = { [key: number]: string };

export default function EnterSecretRecoveryPhraseBody() {
  const { colorMode } = useColorMode();
  const {
    getValues, setValue, watch,
  } = useFormContext<CreateWalletFormValues>();

  const mnemonic = watch('mnemonic');
  const mnemonicValues = watch('mnemonicValues');

  useEffect(() => () => {
    // hide the secret recovery phrase when exit the recovery view
    setValue('showSecretRecoveryPhrase', false);
    setValue('savedSecretRecoveryPhrase', false);
    setValue('confirmSavedsecretRecoveryPhrase', false);
    setValue('mnemonicValues', {});
  }, [setValue]);

  const handleOnInputPaste = (event: any) => {
    event.preventDefault();

    const pasted = event.clipboardData.getData('text/plain');
    const newMnemonicValues: MnemonicValueType = {};
    pasted.split(' ').forEach((v: string, index: number) => {
      newMnemonicValues[index] = v;
    });
    setValue('mnemonicValues', newMnemonicValues);
  };

  const handleOnInputChange = (e: React.ChangeEvent<HTMLInputElement>, index: number) => {
    setValue('mnemonicValues', { ...getValues('mnemonicValues'), [index]: e.target.value });
  };

  return (
    <VStack pt={3} maxH="100%" overflowY="auto" alignItems="left">
      <Heading fontSize="2xl">Enter Your Secret Recovery Phrase</Heading>
      <HStack alignItems="flex-start" height="100%" width="100%">
        <Text fontSize="sm">
          Type your phrase exactly as you saw it on the previous screen
        </Text>
      </HStack>
      <VStack pt={2} width="100%" spacing={2}>
        <SimpleGrid columns={2} gap={4}>
          <VStack key="first-col">
            {mnemonic.slice(0, 6).map((item, index) => (
              <InputGroup key={item} fontWeight="bold" border={borderColor[colorMode]}>
                <InputLeftElement color="teal">{`${index + 1}.`}</InputLeftElement>
                <Input onChange={(e) => handleOnInputChange(e, index)} onPaste={handleOnInputPaste} value={mnemonicValues[index] || ''} variant="outline" key={item} bgColor={secondaryHeaderInputBgColor[colorMode]} fontWeight={600} />
              </InputGroup>
            ))}
          </VStack>
          <VStack key="second-col">
            {mnemonic.slice(6, 12).map((item, index) => (
              <InputGroup size="md" key={item} fontWeight="bold" border={borderColor[colorMode]}>
                <InputLeftElement color="teal">{`${index + 7}.`}</InputLeftElement>
                <Input onChange={(e) => handleOnInputChange(e, index + 6)} onPaste={handleOnInputPaste} value={mnemonicValues[index + 6] || ''} variant="outline" key={item} bgColor={secondaryHeaderInputBgColor[colorMode]} fontWeight={600} />
              </InputGroup>
            ))}
          </VStack>
        </SimpleGrid>
      </VStack>
    </VStack>
  );
}
