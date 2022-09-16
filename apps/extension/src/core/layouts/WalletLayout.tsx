// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import React, { useMemo } from 'react';
import { Box, Grid, useColorMode } from '@chakra-ui/react';
import WalletFooter from 'core/components/WalletFooter';
import WalletHeader from 'core/components/WalletHeader';
import { secondaryBgColor } from 'core/colors';
import styled from '@emotion/styled';

interface WalletLayoutProps {
  children: React.ReactNode;
  hasWalletFooter?: boolean;
  hasWalletHeader?: boolean;
  showAccountCircle?: boolean;
  showBackButton?: boolean;
  title?: string;
}

const BodyDiv = styled(Box)`
  &::-webkit-scrollbar {
    display: none
  }
`;

export default function WalletLayout({
  children,
  hasWalletFooter = true,
  hasWalletHeader = true,
  showAccountCircle = true,
  showBackButton,
  title,
}: WalletLayoutProps) {
  const { colorMode } = useColorMode();

  const templateRows = useMemo(() => {
    if (hasWalletFooter && hasWalletHeader) {
      return '73px 1fr 60px';
    }
    if (hasWalletFooter) {
      return '1fr 40px';
    }
    if (hasWalletHeader) {
      return '73px 1fr';
    }
    return '1fr';
  }, [hasWalletHeader, hasWalletFooter]);

  return (
    <Grid
      height="100%"
      width="100%"
      maxW="100%"
      templateRows={templateRows}
      bgColor={secondaryBgColor[colorMode]}
    >
      {hasWalletHeader ? (
        <WalletHeader
          showAccountCircle={showAccountCircle}
          title={title}
          showBackButton={showBackButton}
        />
      ) : undefined}
      <BodyDiv
        maxH="100%"
        overflowY="auto"
        pb={4}
      >
        {children}
      </BodyDiv>
      {hasWalletFooter ? (
        <WalletFooter />
      ) : undefined}
    </Grid>
  );
}
