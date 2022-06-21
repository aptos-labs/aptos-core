// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { useColorMode, VStack } from '@chakra-ui/react';
import React from 'react';
import HelpHeader from 'core/components/HelpHeader';
import withSimulatedExtensionContainer from 'core/components/WithSimulatedExtensionContainer';
import { secondaryBgColor } from '../core/constants';

/**
 * TODO fill out Help page
 */
function Help() {
  const { colorMode } = useColorMode();
  return (
    <VStack
      bgColor={secondaryBgColor[colorMode]}
      spacing={4}
      width="100%"
      height="100%"
    >
      <HelpHeader />
    </VStack>
  );
}

export default withSimulatedExtensionContainer(Help);
