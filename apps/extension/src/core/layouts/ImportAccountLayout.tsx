// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import React from 'react';
import { Box, Grid, useColorMode } from '@chakra-ui/react';
import { secondaryBgColor } from 'core/colors';
import ImportAccountHeader from 'core/components/ImportAccountHeader';

interface WalletLayoutProps {
  backPage?: string;
  children: React.ReactNode;
  headerValue?: string;
}

export default function ImportAccountLayout({
  backPage,
  children,
  headerValue = 'Import account',
}: WalletLayoutProps) {
  const { colorMode } = useColorMode();

  return (
    <Grid
      height="100%"
      width="100%"
      maxW="100%"
      templateRows="64px 1fr"
      bgColor={secondaryBgColor[colorMode]}
    >
      <ImportAccountHeader backPage={backPage} headerValue={headerValue} />
      <Box maxH="100%" overflowY="auto" pb={4}>
        {children}
      </Box>
    </Grid>
  );
}
