// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import PetraPublicApiProxy from 'shared/petra/proxy';

(window as any).aptos = new PetraPublicApiProxy();
(window as any).petra = (window as any).aptos;
