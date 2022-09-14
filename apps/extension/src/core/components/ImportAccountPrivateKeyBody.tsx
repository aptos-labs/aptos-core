// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  VStack, Heading, Flex, Text, Input, InputGroup, InputRightElement, Button, Box, useColorMode,
} from '@chakra-ui/react';
import { type PrivateKeyFormValues } from 'core/layouts/AddAccountLayout';
import React from 'react';
import { AiOutlineEye } from '@react-icons/all-files/ai/AiOutlineEye';
import { AiOutlineEyeInvisible } from '@react-icons/all-files/ai/AiOutlineEyeInvisible';
import { useFormContext } from 'react-hook-form';
import { buttonBorderColor } from 'core/colors';

interface ImportAccountPrivateKeyBodyProps {
  hasSubmit?: boolean;
  px?: number;
}

export default function ImportAccountPrivateKeyBody({
  hasSubmit,
  px = 4,
}: ImportAccountPrivateKeyBodyProps) {
  const {
    getValues,
    register,
    setValue,
  } = useFormContext<PrivateKeyFormValues>();
  const { colorMode } = useColorMode();

  const showPrivateKey = getValues('showPrivateKey');
  const handleClickShow = () => setValue('showPrivateKey', !getValues('showPrivateKey'));

  return (
    <VStack spacing={4} px={px} pt={4} height="100%">
      <VStack height="100%" width="100%" flex="1">
        <Flex justifyContent="flex-start" width="100%" flexDirection="column">
          <Heading fontSize="xl">Import a private key</Heading>
          <Text fontSize={14}>Access an existing wallet with your private key.</Text>
        </Flex>
        <InputGroup>
          <Input
            variant="filled"
            {...register('privateKey')}
            minLength={1}
            placeholder="Enter private key here"
            height={14}
            type={showPrivateKey ? 'text' : 'password'}
            pr="60px"
          />
          <InputRightElement width="4.5rem" marginTop={1}>
            {showPrivateKey
              ? <AiOutlineEyeInvisible size={28} onClick={handleClickShow} />
              : <AiOutlineEye size={28} onClick={handleClickShow} />}
          </InputRightElement>
        </InputGroup>
      </VStack>
      {
        hasSubmit ? (
          <Box py={2} width="100%" borderTop="1px" pt={2} borderColor={buttonBorderColor[colorMode]}>
            <Button colorScheme="teal" width="100%" type="submit">
              Submit
            </Button>
          </Box>
        ) : null
      }
    </VStack>
  );
}
