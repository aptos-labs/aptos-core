// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { Types } from 'aptos';
import { aptosCoinStoreStructTag } from 'core/constants';

export interface EventHandle {
  counter: Types.U64,
  guid: {
    id: {
      addr: Types.Address,
      creation_num: Types.U64,
    }
  },
}

export interface CoinStoreResourceData {
  coin: { value: Types.U64 },
  deposit_events: EventHandle,
  frozen: boolean,
  withdraw_events: EventHandle
}

export interface CoinStoreResource {
  data: CoinStoreResourceData,
  type: typeof aptosCoinStoreStructTag,
}

export type Resource = CoinStoreResource;
