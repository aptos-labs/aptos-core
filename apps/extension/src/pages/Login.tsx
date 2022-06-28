// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import React from 'react';
import { AptosAccount } from 'aptos';
import { Buffer } from 'buffer';
import {
  useNavigate,
} from 'react-router-dom';
import {
  Box,
  Button,
  Center,
  Flex,
  Heading,
  Input,
  InputGroup,
  InputRightAddon,
  Text,
  useColorMode,
  VStack,
} from '@chakra-ui/react';
import { SubmitHandler, useForm } from 'react-hook-form';
import ChakraLink from 'core/components/ChakraLink';
import useWalletState from 'core/hooks/useWalletState';
import { AptosWhiteLogo, AptosBlackLogo } from 'core/components/AptosLogo';
import withSimulatedExtensionContainer from 'core/components/WithSimulatedExtensionContainer';
import { secondaryBgColor, secondaryErrorMessageColor } from 'core/constants';
import { getAccountResources } from 'core/queries/account';
import AuthLayout from 'core/layouts/AuthLayout';
import { Routes as PageRoutes } from 'core/routes';

export const secondaryTextColor = {
  dark: 'gray.400',
  light: 'gray.500',
};

interface FormValues {
  privateKey: string;
}

function Login() {
  const { colorMode } = useColorMode();
  const { aptosNetwork, updateWalletState } = useWalletState();
  const navigate = useNavigate();
  const {
    formState: { errors }, handleSubmit, register, setError, watch,
  } = useForm<FormValues>();
  const key: string = watch('privateKey');

  const onSubmit: SubmitHandler<Record<string, any>> = async (data, event) => {
    event?.preventDefault();
    try {
      const nonHexKey = (key.startsWith('0x')) ? key.substring(2) : key;
      const encodedKey = Uint8Array.from(Buffer.from(nonHexKey, 'hex'));
      const account = new AptosAccount(encodedKey, undefined);
      const response = await getAccountResources({
        address: account.address().hex(),
        nodeUrl: aptosNetwork,
      });
      if (!response) {
        setError('privateKey', { message: 'Account not found', type: 'custom' });
        return;
      }
      updateWalletState({ aptosAccountState: account });
      navigate('/wallet');
    } catch (err) {
      setError('privateKey', { message: 'Invalid private key', type: 'custom' });
    }
  };

  return (
    <AuthLayout routePath={PageRoutes.login.routePath}>
      <VStack
        bgColor={secondaryBgColor[colorMode]}
        justifyContent="center"
        spacing={4}
        width="100%"
        height="100%"
      >
        <Flex w="100%" flexDir="column">
          <Center>
            <Box width="75px" pb={4}>
              {
              (colorMode === 'dark')
                ? <AptosWhiteLogo />
                : <AptosBlackLogo />
            }
            </Box>
          </Center>
          <Heading textAlign="center">Wallet</Heading>
          <Text
            textAlign="center"
            pb={8}
            color={secondaryTextColor[colorMode]}
            fontSize="lg"
          >
            An Aptos crypto wallet
          </Text>
          <form onSubmit={handleSubmit(onSubmit)}>
            <VStack spacing={4}>
              <Center minW="100%" px={4}>
                <Box>
                  <InputGroup>
                    <Input
                      maxW="350px"
                      {...register('privateKey')}
                      variant="filled"
                      required
                      placeholder="Private key..."
                      autoComplete="off"
                    />
                    <InputRightAddon>
                      <Button type="submit" variant="unstyled">
                        Submit
                      </Button>
                    </InputRightAddon>
                  </InputGroup>
                  <Center>
                    <Text fontSize="xs" color={secondaryErrorMessageColor[colorMode]}>
                      {(errors?.privateKey?.message)}
                    </Text>
                  </Center>
                </Box>
              </Center>
              <ChakraLink to="/create-wallet">
                <Button colorScheme="teal" variant="ghost">
                  Create a new wallet
                </Button>
              </ChakraLink>
            </VStack>
          </form>
        </Flex>
        {/* TODO: Fill this in later */}
        {/* <HStack spacing={2} color={secondaryTextColor[colorMode]}>
        <QuestionIcon />
        <ChakraLink to="/help" fontSize="xs">
          Help
        </ChakraLink>
      </HStack> */}
      </VStack>
    </AuthLayout>
  );
}

export default withSimulatedExtensionContainer({
  Component: Login,
});
