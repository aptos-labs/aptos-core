// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import React from 'react';

import {
  usePermissionRequestContext,
} from '../hooks';
import {
  ConnectRequestPrompt,
  TransactionApprovalPrompt,
  SignatureRequestPrompt,
} from '../components';

export function PermissionsPrompt() {
  const { permissionRequest: { permission } } = usePermissionRequestContext();

  switch (permission.type) {
    case 'connect':
      return <ConnectRequestPrompt />;
    case 'signAndSubmitTransaction':
    case 'signTransaction':
      return <TransactionApprovalPrompt payload={permission.payload} />;
    case 'signMessage':
      return <SignatureRequestPrompt message={permission.message} />;
    default:
      return null;
  }
}

export default PermissionsPrompt;
