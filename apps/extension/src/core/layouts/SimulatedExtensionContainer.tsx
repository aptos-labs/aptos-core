/* eslint-disable no-console */
// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import React, { useState } from 'react';
import {
  Button, Center, Flex, HStack, IconButton, useColorMode, VStack,
} from '@chakra-ui/react';
import { MoonIcon, SunIcon } from '@chakra-ui/icons';
import { useAppState } from 'core/hooks/useAppState';
import {
  ConnectPermissionApproval,
  DappInfo,
  SignTransactionPermissionApproval,
  PermissionHandler,
} from 'shared/permissions';

interface SimulatedExtensionContainerProps {
  children: JSX.Element;
}

export const boxShadow = 'rgba(149, 157, 165, 0.2) 0px 0px 8px 4px';

const extensionDimensions = ['375px', '600px'];
const fullscreenDimensions = ['100vw', 'calc(100vh - 72px)'];

const secondaryFlexBgColor = {
  dark: 'gray.800',
  light: 'gray.100',
};

const secondaryHeaderBgColor = {
  dark: 'gray.700',
  light: 'white',
};

const localOrigin = window && window.location.origin;
const localDappInfo = {
  domain: localOrigin,
  imageURI: window && `${localOrigin}/icon128.png`,
  name: 'Petra Dev',
} as DappInfo;

function DesktopComponent({ children }: SimulatedExtensionContainerProps) {
  const { colorMode, setColorMode } = useColorMode();
  const [
    simulatedDimensions,
    setSimulatedDimensions,
  ] = useState(extensionDimensions);

  const isFullScreen = simulatedDimensions[0] === '100vw';
  const changeDimensionsToExtension = () => setSimulatedDimensions(extensionDimensions);
  const changeDimensionsToFullscreen = () => setSimulatedDimensions(fullscreenDimensions);

  const { activeAccountAddress } = useAppState();
  const testPayload = {
    arguments: [activeAccountAddress, 717],
    function: '0x1::coin::transfer',
    type: 'entry_function_payload',
    type_arguments: ['0x1::aptos_coin::AptosCoin'],
  };

  const promptConnect = async () => {
    const result = await PermissionHandler.requestPermission(
      localDappInfo,
      { type: 'connect' },
    ) as ConnectPermissionApproval;
    console.log('Result', result);
  };

  const promptSignAndSubmitTransaction = async () => {
    const result = await PermissionHandler.requestPermission(
      localDappInfo,
      { payload: testPayload, type: 'signAndSubmitTransaction' },
    ) as SignTransactionPermissionApproval;
    console.log('Result', result);
  };

  const promptSignMessage = async () => {
    const result = await PermissionHandler.requestPermission(
      localDappInfo,
      {
        message: "Si sta come d'autunno, sugli alberi, le foglie",
        type: 'signMessage',
      },
    );
    console.log('Result', result);
  };

  return (
    <VStack w="100vw" h="100vh" spacing={0}>
      <Flex
        flexDirection="row-reverse"
        w="100%"
        py={4}
        bgColor={secondaryHeaderBgColor[colorMode]}
      >
        <HStack spacing={4} pr={4}>
          <Button onClick={promptConnect}>
            Connect
          </Button>
          <Button onClick={promptSignAndSubmitTransaction}>
            Sign transaction
          </Button>
          <Button onClick={promptSignMessage}>
            Sign message
          </Button>
          <Button onClick={changeDimensionsToExtension}>
            Extension
          </Button>
          <Button onClick={changeDimensionsToFullscreen}>
            Full screen
          </Button>
          <IconButton
            aria-label="dark mode"
            icon={colorMode === 'dark' ? <SunIcon /> : <MoonIcon />}
            onClick={() => setColorMode((colorMode === 'dark') ? 'light' : 'dark')}
          />
        </HStack>
      </Flex>
      <Center
        w="100%"
        h="100%"
        bgColor={secondaryFlexBgColor[colorMode]}
      >
        <Center
          maxW={simulatedDimensions[0]}
          maxH={simulatedDimensions[1]}
          w={simulatedDimensions[0]}
          h={simulatedDimensions[1]}
          borderRadius=".5rem"
          boxShadow={isFullScreen ? undefined : boxShadow}
        >
          { children }
        </Center>
      </Center>
    </VStack>
  );
}

export default function SimulatedExtensionContainer({
  children,
}: SimulatedExtensionContainerProps) {
  const isDevelopment = (!process.env.NODE_ENV || process.env.NODE_ENV === 'development');
  return isDevelopment
    ? <DesktopComponent>{ children }</DesktopComponent>
    : children;
}
