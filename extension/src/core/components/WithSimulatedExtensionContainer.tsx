// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import React, { ComponentType, useMemo, useState } from 'react';
import { MoonIcon, SunIcon } from '@chakra-ui/icons';
import {
  Button,
  Center,
  Flex,
  HStack,
  IconButton,
  useColorMode,
  VStack,
} from '@chakra-ui/react';

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

interface WithSimulatedExtensionContainerProps<T> {
  Component: ComponentType<T>,
}

function withSimulatedExtensionContainer<T>({
  Component,
}: WithSimulatedExtensionContainerProps<T>) {
  function HOC(hocProps: T) {
    const { colorMode, setColorMode } = useColorMode();
    const [
      simulatedDimensions,
      setSimulatedDimensions,
    ] = useState(extensionDimensions);

    const isFullScreen = simulatedDimensions[0] === '100vw';

    const changeDimensionsToExtension = () => {
      setSimulatedDimensions(extensionDimensions);
    };

    const changeDimensionsToFullscreen = () => {
      setSimulatedDimensions(fullscreenDimensions);
    };

    const desktopComponent = (
      <VStack w="100vw" h="100vh" spacing={0}>
        <Flex
          flexDirection="row-reverse"
          w="100%"
          py={4}
          bgColor={secondaryHeaderBgColor[colorMode]}
        >
          <HStack spacing={4} pr={4}>
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
            overflow="auto"
            boxShadow={isFullScreen ? undefined : boxShadow}
          >
            <Component {...hocProps} />
          </Center>
        </Center>
      </VStack>
    );

    const extensionComponent = (
      <VStack w="100vw" h="100vh" spacing={0}>
        <Component {...hocProps} />
      </VStack>
    );

    const trueComponent = useMemo(() => {
      if ((!process.env.NODE_ENV || process.env.NODE_ENV === 'development')) {
        return desktopComponent;
      }
      return extensionComponent;
    }, [
      process.env.NODE_ENV,
      desktopComponent,
      extensionComponent,
    ]);

    return trueComponent;
  }
  HOC.displayName = 'withSimulatedExtensionContainerHOC';
  return HOC;
}

export default withSimulatedExtensionContainer;
