// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import React, { StrictMode, useState } from 'react';
import { createRoot } from 'react-dom/client';
import {
  Heading,
  Text,
  Button,
  SimpleGrid,
  extendTheme,
  ChakraProvider,
  VStack,
  StackDivider,
  HStack,
  Image,
} from '@chakra-ui/react';
import { InfoIcon } from '@chakra-ui/icons';
import { PermissionType, PromptMessage } from 'core/types';

const theme = extendTheme({
  initialColorMode: 'light',
  styles: {
    global: {
      'html, body': {
        margin: 0,
        padding: 0,
      },
    },
  },
  useSystemColorMode: false,
});

function PromptState() {
  const [permissionInfo, setPermissionInfo] = useState(undefined);

  chrome.runtime.sendMessage(PromptMessage.LOADED, (response) => {
    setPermissionInfo(response);
  });

  const onApprove = async (data, event) => {
    event?.preventDefault();
    await chrome.runtime.sendMessage(PromptMessage.APPROVED);
    window.close();
  };

  const onCancel = async (data, event) => {
    event?.preventDefault();
    await chrome.runtime.sendMessage(PromptMessage.REJECTED);
    window.close();
  };

  if (permissionInfo) {
    const {
      domain, imageURI, promptType, title,
    } = permissionInfo;

    const permissions = [];
    switch (promptType) {
      case PermissionType.CONNECT:
        permissions.push('View your account address');
        permissions.push('Request your approval for transactions');
        break;
      case PermissionType.SIGN_AND_SUBMIT_TRANSACTION:
      case PermissionType.SIGN_TRANSACTION:
        permissions.push('Sign a transaction');
        break;
      case PermissionType.SIGN_MESSAGE:
        permissions.push('Sign a message');
      default:
        break;
    }

    return (
      <VStack
        divider={<StackDivider borderColor="gray.200" />}
        height="100vh"
        spacing={4}
        padding={4}
        alignItems="stretch"
        width="100%"
        maxW="100%"
      >
        <VStack alignItems="center" width="100%" maxW="100%">
          { imageURI ? (
            <Image
              borderRadius="full"
              boxSize="200px"
              padding={4}
              src={imageURI}
            />
          ) : null}
          {title ? (
            <Heading fontSize="3xl">{title}</Heading>
          ) : null}
          {domain ? (
            <Text fontSize="sm" color="gray.500" wordBreak="break-word">{domain}</Text>
          ) : null}
        </VStack>
        <VStack flexGrow={1} alignItems="flex-start" width="100%" maxW="100%">
          <Text fontSize="md" color="gray.500" wordBreak="break-word">
            This app would like to:
          </Text>
          {permissions.map((permission) => (
            <HStack key={permission}>
              <InfoIcon w={4} h={4} />
              <Text fontSize="sm" wordBreak="break-word">
                {permission}
              </Text>
            </HStack>
          ))}
        </VStack>
        <SimpleGrid flex={0} spacing={4} width="100%" columns={2}>
          <Button onClick={onCancel} >
            Cancel
          </Button>
          <Button colorScheme="teal" onClick={onApprove}>
            Approve
          </Button>
        </SimpleGrid>
      </VStack>
    );
  }
  return null;
}

const root = createRoot(document.getElementById('prompt'));
root.render(
  <ChakraProvider theme={theme}>
    <StrictMode>
      <PromptState />
    </StrictMode>
  </ChakraProvider>,
);
