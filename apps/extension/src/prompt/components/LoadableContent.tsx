// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  Box,
  Center,
  Spinner,
} from '@chakra-ui/react';
import React from 'react';

export interface LoadableContentProps {
  children: JSX.Element | JSX.Element[],
  isLoading: boolean,
}

export function LoadableContent({ children, isLoading }: LoadableContentProps) {
  return (
    <Box position="relative">
      {
        isLoading
          ? (
            <Center
              bgColor="#ffffffb5"
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
