// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { Box, BoxProps, useColorMode } from '@chakra-ui/react';
import React from 'react';
import { permissionRequestTileBgColor } from 'core/colors';

export function Tile({ children, ...rest }: BoxProps) {
  const { colorMode } = useColorMode();
  return (
    <Box
      bgColor={permissionRequestTileBgColor[colorMode]}
      p="21px"
      borderRadius="8px"
      {...rest}
    >
      { children }
    </Box>
  );
}

export default Tile;
