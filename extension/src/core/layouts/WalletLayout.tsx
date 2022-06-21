// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import React from 'react';
import { Box, Grid, useColorMode } from '@chakra-ui/react';
import WalletFooter from 'core/components/WalletFooter';
import WalletHeader from 'core/components/WalletHeader';
import { secondaryBgColor } from 'core/constants';

interface WalletLayoutProps {
  backPage?: string;
  children: React.ReactNode;
}

export default function WalletLayout({
  backPage,
  children,
}: WalletLayoutProps) {
  const { colorMode } = useColorMode();

  return (
    <Grid
      height="100%"
      width="100%"
      maxW="100%"
      templateRows="40px 1fr 50px"
      bgColor={secondaryBgColor[colorMode]}
    >
      <WalletHeader backPage={backPage} />
      <Box maxH="100%" overflowY="auto" pb={4}>
        {children}
      </Box>
      <WalletFooter />
    </Grid>
  );
}
