// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0
import React from 'react';

import {
  Input,
  InputGroup,
  SimpleGrid,
  VStack,
  InputLeftElement,
  useColorMode,
} from '@chakra-ui/react';
import { mnemonicValues } from 'core/constants';
import { secondaryHeaderInputBgColor, mnemonicBorderColor } from 'core/colors';
import { MNEMONIC } from 'core/enums';
import type { UseFormRegister } from 'react-hook-form';

type MnemonicInputProps = {
  register: UseFormRegister<any>;
  setValue: (key: MNEMONIC, value: string) => void;
};

export default function MnemonicInput({ register, setValue }: MnemonicInputProps) {
  const { colorMode } = useColorMode();
  const handleOnInputPaste = (event: any) => {
    event.preventDefault();

    const pasted = event.clipboardData.getData('text/plain');
    pasted.split(' ').forEach((v: never, index: number) => {
      setValue(mnemonicValues[index], v);
    });
  };

  return (
    <SimpleGrid columns={2} gap={4}>
      <VStack key="first-col">
        {mnemonicValues.slice(0, 6).map((mnemonic, index) => (
          <InputGroup key={mnemonic} fontWeight="bold" border={mnemonicBorderColor[colorMode]}>
            <InputLeftElement color="navy.600">{`${index + 1}.`}</InputLeftElement>
            <Input
              {...register(`${mnemonic}`)}
              onPaste={handleOnInputPaste}
              variant="outline"
              key={mnemonic}
              bgColor={secondaryHeaderInputBgColor[colorMode]}
              fontWeight={600}
              height={10}
              autoComplete="off"
            />
          </InputGroup>
        ))}
      </VStack>
      <VStack key="second-col">
        {mnemonicValues.slice(6, 12).map((mnemonic, index) => (
          <InputGroup
            size="md"
            key={mnemonic}
            fontWeight="bold"
            border={mnemonicBorderColor[colorMode]}
          >
            <InputLeftElement color="navy.600">{`${index + 7}.`}</InputLeftElement>
            <Input
              {...register(mnemonic)}
              onPaste={handleOnInputPaste}
              variant="outline"
              key={mnemonic}
              height={10}
              bgColor={secondaryHeaderInputBgColor[colorMode]}
              fontWeight={600}
              autoComplete="off"
            />
          </InputGroup>
        ))}
      </VStack>
    </SimpleGrid>
  );
}
