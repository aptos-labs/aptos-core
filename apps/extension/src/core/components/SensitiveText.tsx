// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import React from 'react';
import {
  Input, useColorMode,
} from '@chakra-ui/react';
import {
  secondaryTextColor,
} from 'core/colors';

interface SensitiveTextProps {
  height?: number;
  value: string;
}

function SensitiveText({
  height = 12, value,
}: SensitiveTextProps) {
  const { colorMode } = useColorMode();
  return (
    <Input
      marginTop={4}
      height={height}
      color={secondaryTextColor[colorMode]}
      readOnly
      variant="filled"
      fontSize="sm"
      value={value}
    />
  );
}

export default SensitiveText;
