// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import React from 'react';
import Routes from 'core/routes';
import {
  Box,
  Button,
  Center,
  FormControl,
  FormErrorMessage,
  FormLabel,
  Grid,
  Input,
  Spinner,
  Text,
  useColorMode,
  VStack,
} from '@chakra-ui/react';
import { secondaryBgColor, secondaryBorderColor } from 'core/colors';
import ChakraLink from 'core/components/ChakraLink';
import { ChevronLeftIcon } from '@chakra-ui/icons';
import { SubmitHandler, useForm } from 'react-hook-form';
import { useNavigate } from 'react-router-dom';
import { useNetworks } from 'core/hooks/useNetworks';
import { DefaultNetworks, defaultNetworks } from 'shared/types';
import { useNodeStatus } from 'core/queries/network';
import useDebounce from 'core/hooks/useDebounce';
import { addNetworkToast } from 'core/components/Toast';

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
  const {
    addNetwork,
    networks,
  } = useNetworks();
  const navigate = useNavigate();
  const {
    formState: { errors, isValid },
    handleSubmit,
    register,
    watch,
  } = useForm<AddNetworkFormData>({
    mode: 'onChange',
  });

  // Make sure to check node status only when format is right
  const shouldCheckNodeStatus = errors.nodeUrl === undefined;
  const debouncedNodeUrl = useDebounce(watch('nodeUrl'), 300);
  const {
    isLoading: isNodeStatusLoading,
    isNodeAvailable,
  } = useNodeStatus(debouncedNodeUrl, {
    enabled: shouldCheckNodeStatus,
    refetchInterval: 5000,
  });

  const nameValidators = {
    ...requiredValidator,
    maxLength: {
      message: 'Name is too long',
      value: 30,
    },
    validate: {
      unique: (name: string) => !networks || !(name in networks),
    },
  };

  const nodeUrlValidators = {
    ...requiredValidator,
    ...urlValidator,
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
    const shouldSwitch = isNodeAvailable === true;
    const network = {
      faucetUrl: faucetUrl || undefined,
      name,
      nodeUrl,
    };

    await addNetwork(network, shouldSwitch);
    addNetworkToast(shouldSwitch ? network.name : undefined);
    navigate(Routes.network.path);
  };

  return (
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
              <FormControl isRequired isInvalid={errors.name !== undefined}>
                <FormLabel>Name</FormLabel>
                <Input
                  placeholder="Custom network"
                  {...register('name', nameValidators)}
                />
                <FormErrorMessage>
                  {
                    errors.name?.type === 'unique'
                      ? 'A network with this name already exists'
                      : errors.name?.message
                  }
                </FormErrorMessage>
              </FormControl>
              <FormControl isRequired isInvalid={errors.nodeUrl !== undefined}>
                <FormLabel>Node URL</FormLabel>
                <Input
                  placeholder={referenceNetwork.nodeUrl}
                  {...register('nodeUrl', nodeUrlValidators)}
                />
                <FormErrorMessage>
                  {
                    errors.nodeUrl?.type === 'unique'
                      ? 'A network with this nodeUrl already exists'
                      : errors.nodeUrl?.message
                  }

                </FormErrorMessage>
                {
                  !errors.nodeUrl ? (
                    <Box mt={2} fontSize="sm" lineHeight="normal">
                      { isNodeStatusLoading ? <Spinner size="sm" /> : null }
                      { isNodeAvailable === false ? <Text color="yellow.500">Node is not available</Text> : null }
                    </Box>
                  ) : null
                }
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
  );
}
