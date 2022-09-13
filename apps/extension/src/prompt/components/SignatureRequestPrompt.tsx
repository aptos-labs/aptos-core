// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { Heading, Text } from '@chakra-ui/react';
import React, { useEffect } from 'react';

import { PermissionPromptLayout } from './PermissionPromptLayout';
import { usePermissionRequestContext } from '../hooks';
import { Tile } from './Tile';

interface SignatureRequestPromptProps {
  message: string,
}

export function SignatureRequestPrompt({ message }: SignatureRequestPromptProps) {
  const { setApprovalState } = usePermissionRequestContext();

  useEffect(() => {
    setApprovalState({ canApprove: true });
  }, [setApprovalState]);

  return (
    <PermissionPromptLayout title="Signature request">
      <Tile>
        <Heading size="sm" lineHeight="24px" mb="4px">
          Message
        </Heading>
        <Text fontSize="sm" lineHeight="20px">
          {message}
        </Text>
      </Tile>
    </PermissionPromptLayout>
  );
}

export default SignatureRequestPrompt;
