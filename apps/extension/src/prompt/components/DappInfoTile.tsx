// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  Heading,
  Image,
  Square,
  Text,
  VStack,
} from '@chakra-ui/react';
import React, { useMemo } from 'react';

import { useActiveAccount } from 'core/hooks/useAccounts';
import collapseHexString from 'core/utils/hex';
import { usePermissionRequestContext } from '../hooks';
import { Tile } from './Tile';

export function DappInfoTile() {
  const { permissionRequest: { dappInfo } } = usePermissionRequestContext();
  const { activeAccountAddress } = useActiveAccount();

  return useMemo(() => (
    <Tile mt="33px" position="relative">
      { /* Wrapper for transparent icons */ }
      <Square
        size="51px"
        borderRadius={8}
        position="absolute"
        top="-25.5px"
        left="calc(50% - 25.5px)"
      >
        <Image src={dappInfo.imageURI} />
      </Square>
      <VStack
        pt="11px"
        alignItems="center"
        spacing={0}
      >
        <Heading size="sm" lineHeight="24px">
          {dappInfo.domain}
        </Heading>
        <Text fontSize="md" lineHeight="24px" color="gray.600">
          {`Approve with ${collapseHexString(activeAccountAddress, 8)}`}
        </Text>
      </VStack>
    </Tile>
  ), [dappInfo, activeAccountAddress]);
}

export default DappInfoTile;
