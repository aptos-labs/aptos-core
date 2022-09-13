// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import * as ExtensionImpl from './extension';
import * as WindowImpl from './window';
import {
  PermissionHandler as PermissionHandlerInterface,
  PermissionResponseError,
} from './shared';

const isDevelopment = chrome.runtime === undefined;

const PermissionHandler: PermissionHandlerInterface = isDevelopment
  ? WindowImpl
  : ExtensionImpl;

export { PermissionHandler, PermissionResponseError };
