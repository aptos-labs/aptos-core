// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import React from 'react';
import AuthLayout from 'core/layouts/AuthLayout';
import Routes, { Routes as PageRoutes } from 'core/routes';
import {
  Box,
  Button,
  Center,
  FormControl,
  FormErrorMessage,
  FormLabel,
  Grid,
  Input,
  Text,
  useColorMode,
  VStack,
} from '@chakra-ui/react';
import { secondaryBgColor, secondaryBorderColor } from 'core/colors';
import ChakraLink from 'core/components/ChakraLink';
import { ChevronLeftIcon } from '@chakra-ui/icons';
import { SubmitHandler, useForm } from 'react-hook-form';
import { useNavigate } from 'react-router-dom';
import useGlobalStateContext, { defaultNetworks, DefaultNetworks } from 'core/hooks/useGlobalState';

interface AddNetworkFormData {
  faucetUrl?: string,
  name: string,
  nodeUrl: string,
}

const requiredValidator = {
  required: 'This is required',
};

const urlValidator = {
  pattern: {
    message: 'Not a valid URL',
    value: /^(http|https):\/(\/([A-z\d-]+\.)*([A-z\d-]+))+(:\d+)?$/,
  },
};

const referenceNetwork = defaultNetworks[DefaultNetworks.Devnet];

export default function AddNetwork() {
  const { colorMode } = useColorMode();
  const { addNetwork, networks } = useGlobalStateContext();
  const navigate = useNavigate();
  const {
    formState: { errors, isValid },
    handleSubmit,
    register,
  } = useForm<AddNetworkFormData>({
    mode: 'onChange',
  });

  const uniqueNameValidator = {
    validate: {
      unique: (name: string) => !networks || !(name in networks),
    },
  };

  const uniqueNetworkValidator = {
    validate: {
      unique: (nodeUrl: string) => !networks || !Object.values(networks)
        .some((n) => n.nodeUrl === nodeUrl),
    },
  };

  const onSubmit: SubmitHandler<AddNetworkFormData> = async ({
    faucetUrl,
    name,
    nodeUrl,
  }, event) => {
    event?.preventDefault();
    const network = {
      faucetUrl: faucetUrl || undefined,
      name,
      nodeUrl,
    };
    await addNetwork(network);
    navigate(Routes.network.path);
  };

  return (
    <AuthLayout routePath={PageRoutes.addNetwork.path}>
      <Grid
        height="100%"
        width="100%"
        maxW="100%"
        templateRows="64px 1fr"
        bgColor={secondaryBgColor[colorMode]}
      >
        <Grid
          maxW="100%"
          width="100%"
          py={4}
          height="64px"
          templateColumns="40px 1fr 40px"
          borderBottomColor={secondaryBorderColor[colorMode]}
          borderBottomWidth="1px"
        >
          <Center>
            <ChakraLink to={Routes.wallet.path}>
              <ChevronLeftIcon fontSize="xl" aria-label={Routes.wallet.path} />
            </ChakraLink>
          </Center>
          <Center width="100%">
            <Text fontSize="md" fontWeight={600}>
              Add network
            </Text>
          </Center>
          <Box />
        </Grid>
        <Box maxH="100%" overflowY="auto" pb={4}>
          <Box width="100%" height="100%" px={4}>
            <form onSubmit={handleSubmit(onSubmit)}>
              <VStack spacing={4} px={4} pt={4}>
                <FormControl isInvalid={errors.name !== undefined}>
                  <FormLabel>Name</FormLabel>
                  <Input
                    placeholder="Custom network"
                    {...register('name', { ...requiredValidator, ...uniqueNameValidator })}
                  />
                  <FormErrorMessage>
                    {
                      errors.name?.type === 'unique'
                        ? 'A network with this name already exists'
                        : errors.name?.message
                    }
                  </FormErrorMessage>
                </FormControl>
                <FormControl isInvalid={errors.nodeUrl !== undefined}>
                  <FormLabel>Node URL</FormLabel>
                  <Input
                    placeholder={referenceNetwork.nodeUrl}
                    {...register('nodeUrl', {
                      ...requiredValidator,
                      ...urlValidator,
                      ...uniqueNetworkValidator,
                    })}
                  />
                  <FormErrorMessage>
                    {
                      errors.nodeUrl?.type === 'unique'
                        ? 'A network with this nodeUrl already exists'
                        : errors.nodeUrl?.message
                    }
                  </FormErrorMessage>
                </FormControl>
                <FormControl isInvalid={errors.faucetUrl !== undefined}>
                  <FormLabel>Faucet URL (optional)</FormLabel>
                  <Input
                    placeholder={referenceNetwork.faucetUrl}
                    {...register('faucetUrl', { ...urlValidator })}
                  />
                  <FormErrorMessage>{ errors.faucetUrl?.message }</FormErrorMessage>
                </FormControl>
                <Button
                  colorScheme="teal"
                  width="100%"
                  type="submit"
                  isDisabled={!isValid}
                >
                  Add
                </Button>
              </VStack>
            </form>
          </Box>
        </Box>
      </Grid>
    </AuthLayout>
  );
}
