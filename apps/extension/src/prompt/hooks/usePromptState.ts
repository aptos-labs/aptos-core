// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import constate from 'constate';
import { useEffect, useState } from 'react';
import {
  PermissionHandler,
  PermissionRequest,
  PermissionResponseStatus,
} from 'shared/permissions';
import { PersistentStorage } from 'shared/storage';

export const [PromptStateProvider, usePromptState] = constate(() => {
  const [permissionRequest, setPermissionRequest] = useState<PermissionRequest>();

  // Initialize state by querying storage
  useEffect(() => {
    PersistentStorage.get(['permissionRequest']).then((initialState) => {
      setPermissionRequest(initialState.permissionRequest);
    });
  }, []);

  // Keep state up to date by listening to storage changes
  useEffect(() => PersistentStorage.onChange((changes) => {
    const newRequest = changes?.permissionRequest?.newValue;
    if (newRequest !== undefined) {
      setPermissionRequest((prevRequest) => {
        if (prevRequest) {
          PermissionHandler.sendPermissionResponse({
            id: prevRequest.id,
            status: PermissionResponseStatus.Timeout,
          }).then();
        }
        return newRequest;
      });
    }
  }), []);

  return {
    permissionRequest,
  };
});
