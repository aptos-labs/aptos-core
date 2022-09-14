// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import styled from '@emotion/styled';
import React from 'react';
import {
  Box,
  Button,
  HStack,
  Text,
  VStack,
  useColorMode,
} from '@chakra-ui/react';

import AccountCircle from 'core/components/AccountCircle';
import {
  permissionRequestBgColor,
  permissionRequestLayoutBgColor,
  secondaryBorderColor,
} from 'core/colors';
import { usePermissionRequestContext } from '../hooks';
import { DappInfoTile } from './DappInfoTile';

const FooterButton = styled(Button)`
  height: 48px;
  font-size: 18px;
  line-height: 28px;
`;

const hiddenScrollbarCss = { '&::-webkit-scrollbar': { display: 'none' } };

interface PermissionRequestHeaderProps {
  title: string,
}

function PermissionRequestHeader({ title }: PermissionRequestHeaderProps) {
  const { colorMode } = useColorMode();

  return (
    <HStack
      height="84px"
      minHeight="84px"
      borderBottomColor={secondaryBorderColor[colorMode]}
      borderBottomWidth="1px"
      justifyContent="space-between"
      padding={6}
      bgColor={permissionRequestLayoutBgColor[colorMode]}
    >
      <Text fontSize={20} fontWeight="bold">
        {title}
      </Text>
      <AccountCircle />
    </HStack>
  );
}

function PermissionRequestFooter() {
  const { colorMode } = useColorMode();
  const { approve, canApprove, reject } = usePermissionRequestContext();

  const onApprovePressed = async (event: React.MouseEvent) => {
    event?.preventDefault();
    await approve();
    window.close();
  };

  const onCancelPressed = async (event: React.MouseEvent) => {
    event?.preventDefault();
    await reject();
    window.close();
  };

  return (
    <HStack
      height="75px"
      minHeight="75px"
      bgColor={permissionRequestLayoutBgColor[colorMode]}
      px="24px"
      spacing="8px"
      borderTopColor={secondaryBorderColor[colorMode]}
      borderTopWidth="1px"
    >
      <FooterButton w="50%" variant="outline" onClick={onCancelPressed}>
        Cancel
      </FooterButton>
      <FooterButton w="50%" colorScheme="teal" isDisabled={!canApprove} onClick={onApprovePressed}>
        Approve
      </FooterButton>
    </HStack>
  );
}

export interface PermissionPromptLayoutProps {
  children: JSX.Element | JSX.Element[],
  title: string,
}

export function PermissionPromptLayout({ children, title }: PermissionPromptLayoutProps) {
  const { colorMode } = useColorMode();

  return (
    <VStack
      h="100%"
      w="100%"
      spacing={0}
      alignItems="stretch"
    >
      <PermissionRequestHeader title={title} />
      <Box
        flexGrow={1}
        overflowY="scroll"
        css={hiddenScrollbarCss}
        bgColor={permissionRequestBgColor[colorMode]}
      >
        <VStack
          width="100%"
          p="21px"
          spacing="16px"
          alignItems="stretch"
          flexGrow={1}
        >
          <DappInfoTile />
          { children }
        </VStack>
      </Box>
      <PermissionRequestFooter />
    </VStack>
  );
}

export default PermissionPromptLayout;
