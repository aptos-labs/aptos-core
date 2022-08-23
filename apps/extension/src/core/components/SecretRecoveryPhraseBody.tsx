// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  Center,
  Checkbox,
  Heading,
  Input,
  InputGroup,
  InputLeftAddon,
  SimpleGrid,
  Text,
  useColorMode,
  VStack,
} from '@chakra-ui/react';
import { secondaryTextColor } from 'core/colors';
import { type CreateWalletFormValues } from 'core/layouts/CreateWalletLayout';
import React from 'react';
import { useFormContext } from 'react-hook-form';

export default function SecretRecoveryPhraseBody() {
  const { colorMode } = useColorMode();
  const { register, watch } = useFormContext<CreateWalletFormValues>();

  const mnemonic = watch('mnemonic');

  return (
    <VStack pt={8} maxH="100%" overflowY="auto">
      <Heading fontSize="3xl">Secret recovery phrase</Heading>
      <Text
        fontSize="md"
        textAlign="center"
      >
        This phrase is the ONLY way to recover your wallet. Do NOT share it with anyone!
      </Text>
      <VStack pt={8} width="100%">
        <SimpleGrid columns={2} gap={4}>
          <VStack>
            {mnemonic.slice(0, 6).map((item, index) => (
              <InputGroup size="sm" key={item}>
                <InputLeftAddon>{index + 1}</InputLeftAddon>
                <Input readOnly variant="outline" value={item} key={item} />
              </InputGroup>
            ))}
          </VStack>
          <VStack>
            {mnemonic.slice(6, 12).map((item, index) => (
              <InputGroup size="sm" key={item}>
                <InputLeftAddon>{index + 7}</InputLeftAddon>
                <Input readOnly variant="outline" value={item} key={item} />
              </InputGroup>
            ))}
          </VStack>
        </SimpleGrid>
      </VStack>
      <Center width="100%" pt={4}>
        <Checkbox
          colorScheme="teal"
          value="terms"
          color={secondaryTextColor[colorMode]}
          {...register('secretRecoveryPhrase')}
        >
          I saved my Secret Recovery Phrase
        </Checkbox>
      </Center>
    </VStack>
  );
}
