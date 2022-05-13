// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import React, { useEffect } from 'react';
import { Box, Grid, useColorMode } from '@chakra-ui/react';
import { useNavigate } from 'react-router-dom';
import useWalletState from 'core/hooks/useWalletState';
import WalletFooter from 'core/components/WalletFooter';
import WalletHeader from 'core/components/WalletHeader';
import { secondaryBgColor } from 'core/constants';

interface WalletLayoutProps {
  children: React.ReactNode
}

export default function WalletLayout({
  children,
}: WalletLayoutProps) {
  const { colorMode } = useColorMode();
  const { aptosAccount } = useWalletState();
  const navigate = useNavigate();

  useEffect(() => {
    if (typeof window !== 'undefined') {
      if (!aptosAccount) {
        navigate('/');
      }
    }
  }, []);

  return (
    <Grid
      height="100%"
      width="100%"
      maxW="100%"
      templateRows="30px 1fr 50px"
      bgColor={secondaryBgColor[colorMode]}
    >
      <WalletHeader />
      <Box maxH="100%" overflowY="auto" pb={4}>
        {children}
      </Box>
      <WalletFooter />
    </Grid>
  );
}
