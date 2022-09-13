// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { MdCheckCircle } from '@react-icons/all-files/md/MdCheckCircle';
import {
  Heading,
  List,
  ListIcon,
  ListItem,
} from '@chakra-ui/react';
import React, { useEffect } from 'react';

import { usePermissionRequestContext } from '../hooks';
import { PermissionPromptLayout } from './PermissionPromptLayout';
import { Tile } from './Tile';

export function ConnectRequestPrompt() {
  const { setApprovalState } = usePermissionRequestContext();

  useEffect(() => {
    setApprovalState({ canApprove: true });
  }, [setApprovalState]);

  return (
    <PermissionPromptLayout title="Signature request">
      <Tile>
        <Heading size="sm" lineHeight="24px" mb="8px">
          This app would like to:
        </Heading>
        <List fontSize="sm" lineHeight="20px" spacing="4px">
          <ListItem>
            <ListIcon as={MdCheckCircle} color="green.500" />
            View your balance and activity
          </ListItem>
          <ListItem>
            <ListIcon as={MdCheckCircle} color="green.500" />
            Request approval for transactions
          </ListItem>
        </List>
      </Tile>
    </PermissionPromptLayout>
  );
}

export default ConnectRequestPrompt;
