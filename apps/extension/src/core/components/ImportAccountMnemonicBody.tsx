// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  Button,
  Input,
  InputGroup,
  InputLeftAddon,
  SimpleGrid,
  VStack,
} from '@chakra-ui/react';
import React from 'react';
import { useFormContext } from 'react-hook-form';
import { type MnemonicFormValues } from 'core/layouts/AddAccountLayout';

export default function ImportAccountMnemonicBody() {
  const {
    register,
  } = useFormContext<MnemonicFormValues>();

  return (
    <VStack spacing={4} px={4} pt={4}>
      <VStack width="100%">
        <SimpleGrid columns={2} gap={4}>
          <VStack>
            <InputGroup size="sm">
              <InputLeftAddon>1</InputLeftAddon>
              <Input minLength={1} {...register('mnemonic-a')} isRequired variant="outline" />
            </InputGroup>
            <InputGroup size="sm">
              <InputLeftAddon>2</InputLeftAddon>
              <Input minLength={1} {...register('mnemonic-b')} isRequired variant="outline" />
            </InputGroup>
            <InputGroup size="sm">
              <InputLeftAddon>3</InputLeftAddon>
              <Input minLength={1} {...register('mnemonic-c')} isRequired variant="outline" />
            </InputGroup>
            <InputGroup size="sm">
              <InputLeftAddon>4</InputLeftAddon>
              <Input minLength={1} {...register('mnemonic-d')} isRequired variant="outline" />
            </InputGroup>
            <InputGroup size="sm">
              <InputLeftAddon>5</InputLeftAddon>
              <Input minLength={1} {...register('mnemonic-e')} isRequired variant="outline" />
            </InputGroup>
            <InputGroup size="sm">
              <InputLeftAddon>6</InputLeftAddon>
              <Input minLength={1} {...register('mnemonic-f')} isRequired variant="outline" />
            </InputGroup>
          </VStack>
          <VStack>
            <InputGroup size="sm">
              <InputLeftAddon>7</InputLeftAddon>
              <Input minLength={1} {...register('mnemonic-g')} isRequired variant="outline" />
            </InputGroup>
            <InputGroup size="sm">
              <InputLeftAddon>8</InputLeftAddon>
              <Input minLength={1} {...register('mnemonic-h')} isRequired variant="outline" />
            </InputGroup>
            <InputGroup size="sm">
              <InputLeftAddon>9</InputLeftAddon>
              <Input minLength={1} {...register('mnemonic-i')} isRequired variant="outline" />
            </InputGroup>
            <InputGroup size="sm">
              <InputLeftAddon>10</InputLeftAddon>
              <Input minLength={1} {...register('mnemonic-j')} isRequired variant="outline" />
            </InputGroup>
            <InputGroup size="sm">
              <InputLeftAddon>11</InputLeftAddon>
              <Input minLength={1} {...register('mnemonic-k')} isRequired variant="outline" />
            </InputGroup>
            <InputGroup size="sm">
              <InputLeftAddon>12</InputLeftAddon>
              <Input minLength={1} {...register('mnemonic-l')} isRequired variant="outline" />
            </InputGroup>
          </VStack>
        </SimpleGrid>
      </VStack>
      <Button colorScheme="teal" width="100%" type="submit">
        Submit
      </Button>
    </VStack>
  );
}
