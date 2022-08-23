// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  Button,
  Center,
  Checkbox,
  Heading,
  Icon,
  Input,
  InputGroup,
  InputRightElement,
  Tag,
  Text,
  useColorMode,
  VStack,
} from '@chakra-ui/react';
import { AiFillEye } from '@react-icons/all-files/ai/AiFillEye';
import { AiFillEyeInvisible } from '@react-icons/all-files/ai/AiFillEyeInvisible';
import { secondaryTextColor } from 'core/colors';
import React, { useState } from 'react';
import { useFormContext } from 'react-hook-form';
import { zxcvbn, zxcvbnOptions } from '@zxcvbn-ts/core';
import zxcvbnCommonPackage from '@zxcvbn-ts/language-common';
import zxcvbnEnPackage from '@zxcvbn-ts/language-en';
import { type CreateWalletFormValues } from 'core/layouts/CreateWalletLayout';

export const passwordOptions = {
  dictionary: {
    ...zxcvbnCommonPackage.dictionary,
    ...zxcvbnEnPackage.dictionary,
  },
  graphs: zxcvbnCommonPackage.adjacencyGraphs,
  translations: zxcvbnEnPackage.translations,
};

zxcvbnOptions.setOptions(passwordOptions);

export const passwordScoreArray = Object.freeze([
  {
    color: 'red',
    value: 'very weak',
  },
  {
    color: 'red',
    value: 'weak',
  },
  {
    color: 'yellow',
    value: 'medium',
  },
  {
    color: 'green',
    value: 'strong',
  },
  {
    color: 'green',
    value: 'very strong',
  },
] as const);

export default function CreatePasswordBody() {
  const { colorMode } = useColorMode();
  const { register, watch } = useFormContext<CreateWalletFormValues>();
  const [show, setShow] = useState(false);
  const handleClick = () => setShow(!show);

  const initialPassword = watch('initialPassword');
  const result = zxcvbn(initialPassword);
  const passwordScore = result.score;

  return (
    <VStack pt={20}>
      <Heading fontSize="3xl">Create a password</Heading>
      <Text fontSize="md">You will use this to unlock your wallet</Text>
      <VStack pt={8} width="100%">
        <InputGroup>
          <Input
            autoFocus
            autoComplete="false"
            variant="filled"
            type={show ? 'text' : 'password'}
            placeholder="Enter password..."
            maxLength={64}
            {...register('initialPassword')}
          />
          <InputRightElement width="3rem">
            <Button
              tabIndex={-3}
              borderRadius="100%"
              variant="ghost"
              h="1.75rem"
              size="sm"
              onClick={handleClick}
            >
              {show
                ? <Icon as={AiFillEyeInvisible} />
                : <Icon as={AiFillEye} />}
            </Button>
          </InputRightElement>
        </InputGroup>
        <InputGroup>
          <Input
            autoComplete="false"
            variant="filled"
            type={show ? 'text' : 'password'}
            placeholder="Confirm password..."
            maxLength={64}
            {...register('confirmPassword')}
          />
          <InputRightElement width="3rem">
            <Button
              tabIndex={-3}
              borderRadius="100%"
              variant="ghost"
              h="1.75rem"
              size="sm"
              onClick={handleClick}
            >
              {show
                ? <Icon as={AiFillEyeInvisible} />
                : <Icon as={AiFillEye} />}
            </Button>
          </InputRightElement>
        </InputGroup>
      </VStack>
      <Text pt={4} fontSize="md">
        Password strength:&nbsp;
        <Tag colorScheme={passwordScoreArray[passwordScore]?.color}>
          {passwordScoreArray[passwordScore]?.value}
        </Tag>
      </Text>
      <Center width="100%" pt={4}>
        <Checkbox
          colorScheme="teal"
          value="terms"
          color={secondaryTextColor[colorMode]}
          {...register('termsOfService')}
        >
          I agree to the
          {' '}
          <Button
            as="a"
            href="https://petra.app/Wallet_Terms.pdf"
            color="teal.500"
            target="_blank"
            rel="noreferrer"
            variant="link"
          >
            Terms of Service
          </Button>
        </Checkbox>
      </Center>
    </VStack>
  );
}
