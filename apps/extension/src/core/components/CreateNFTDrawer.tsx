// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { AddIcon, ExternalLinkIcon } from '@chakra-ui/icons';
import {
  Button,
  Drawer,
  DrawerBody,
  DrawerContent,
  DrawerFooter,
  DrawerHeader,
  DrawerOverlay,
  Flex,
  FormControl,
  FormLabel,
  Grid,
  Input,
  Text,
  useColorMode,
  useDisclosure,
  VStack,
} from '@chakra-ui/react';
import React from 'react';
import { SubmitHandler, useForm } from 'react-hook-form';
import useWalletState from 'core/hooks/useWalletState';
import { secondaryTextColor } from 'pages/Login';
import { useCreateTokenAndCollection } from 'core/mutations/collectibles';

// eslint-disable-next-line global-require
window.Buffer = window.Buffer || require('buffer').Buffer;

export default function CreateNFTModal() {
  const { colorMode } = useColorMode();
  const { isOpen, onClose, onOpen } = useDisclosure();
  const { handleSubmit, register, watch } = useForm();
  const { aptosAccount, aptosNetwork } = useWalletState();

  const collectionName: string | undefined = watch('collectionName');
  const tokenName: string | undefined = watch('tokenName');
  const description: string | undefined = watch('description');
  const supply = Number(watch('supply') || 1);
  const uri: string | undefined = watch('uri');

  const {
    error,
    isError,
    isLoading,
    mutateAsync: createTokenAndCollectionOnClick,
  } = useCreateTokenAndCollection();

  const errorMessage = error?.response?.data?.message;

  const onSubmit: SubmitHandler<Record<string, any>> = async (_data, event) => {
    event?.preventDefault();
    await createTokenAndCollectionOnClick({
      account: aptosAccount,
      collectionName,
      description,
      name: tokenName,
      nodeUrl: aptosNetwork,
      supply,
      uri,
    });
    onClose();
  };

  return (
    <>
      <Button size="xs" onClick={onOpen} leftIcon={<AddIcon fontSize="xs" />}>
        New
      </Button>
      <Drawer
        isOpen={isOpen}
        onClose={onClose}
        placement="bottom"
      >
        <DrawerOverlay />
        <DrawerContent>
          <form onSubmit={handleSubmit(onSubmit)}>
            <DrawerHeader>Create a collectible</DrawerHeader>
            <DrawerBody overflowY="auto" maxH="450px">
              <VStack>
                <FormControl isRequired>
                  <FormLabel fontWeight={400} color={secondaryTextColor[colorMode]}>
                    Collection name
                  </FormLabel>
                  <Input
                    {...register('collectionName')}
                    variant="filled"
                    required
                    maxLength={100}
                  />
                </FormControl>
                <FormControl isRequired>
                  <FormLabel fontWeight={400} color={secondaryTextColor[colorMode]}>
                    Token name
                  </FormLabel>
                  <Input
                    {...register('tokenName')}
                    variant="filled"
                    required
                    maxLength={100}
                  />
                </FormControl>
                <FormControl isRequired>
                  <FormLabel fontWeight={400} color={secondaryTextColor[colorMode]}>
                    Description
                  </FormLabel>
                  <Input
                    {...register('description')}
                    variant="filled"
                    required
                    maxLength={3000}
                    placeholder="A description of your collection"
                  />
                </FormControl>
                <FormControl isRequired>
                  <FormLabel fontWeight={400} color={secondaryTextColor[colorMode]}>
                    Supply
                  </FormLabel>
                  <Input
                    {...register('supply')}
                    variant="filled"
                    type="number"
                    min={1}
                    required
                    defaultValue={1}
                    max={1e9}
                  />
                </FormControl>
                <FormControl isRequired>
                  <Grid templateColumns="1fr 200px">
                    <Flex>
                      <FormLabel
                        fontWeight={400}
                        color={secondaryTextColor[colorMode]}
                      >
                        Uri
                      </FormLabel>
                    </Flex>
                    <Flex justifyContent="flex-end">
                      <Button
                        fontSize="md"
                        fontWeight={400}
                        height="24px"
                        as="a"
                        target="_blank"
                        rightIcon={<ExternalLinkIcon />}
                        variant="unstyled"
                        cursor="pointer"
                        color={secondaryTextColor[colorMode]}
                        href="https://github.com/aptos-labs/aptos-core/blob/8b826d88b0f17255a753214ede48cbc44e484a97/ecosystem/web-wallet/src/core/types/TokenMetadata.ts"
                      >
                        (Metadata structure)
                      </Button>
                    </Flex>
                  </Grid>
                  <Input
                    {...register('uri')}
                    variant="filled"
                    required
                    maxLength={300}
                    placeholder="Arweave, IPFS, or S3 uri"
                  />
                </FormControl>
                {
                  (isError)
                    ? (
                      <Text color="red.400" maxW="100%">
                        {errorMessage}
                      </Text>
                    )
                    : undefined
                }
              </VStack>
            </DrawerBody>
            <DrawerFooter>
              <Button isLoading={isLoading} colorScheme="blue" mr={3} type="submit">
                Submit
              </Button>
              <Button variant="ghost" onClick={onClose}>Close</Button>
            </DrawerFooter>
          </form>
        </DrawerContent>
      </Drawer>
    </>
  );
}
