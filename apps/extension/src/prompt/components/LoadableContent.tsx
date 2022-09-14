// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  Box,
  Center,
  Spinner, useColorMode,
} from '@chakra-ui/react';
import React from 'react';
import { permissionRequestLoadingOverlayColor } from 'core/colors';

export interface LoadableContentProps {
  children: JSX.Element | JSX.Element[],
  isLoading: boolean,
}

export function LoadableContent({ children, isLoading }: LoadableContentProps) {
  const { colorMode } = useColorMode();
  return (
    <Box position="relative">
      {
        isLoading
          ? (
            <Center
              bgColor={permissionRequestLoadingOverlayColor[colorMode]}
              w="100%"
              h="100%"
              position="absolute"
            >
              <Spinner />
            </Center>
          )
          : null
      }
      { children }
    </Box>
  );
}

export default LoadableContent;
