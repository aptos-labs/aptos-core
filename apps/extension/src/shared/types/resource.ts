// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { Types } from 'aptos';
import {
  aptosCoinStoreStructTag,
  aptosStakePoolStructTag,
} from 'core/constants';

export interface Guid {
  id: {
    addr: Types.Address,
    creation_num: Types.U64,
  },
}

export interface EventHandle {
  counter: Types.U64,
  guid: Guid,
}

export interface CoinStoreResourceData {
  coin: { value: Types.U64 },
  deposit_events: EventHandle,
  frozen: boolean,
  withdraw_events: EventHandle
}

export interface CoinStoreResource {
  data: CoinStoreResourceData,
  type: typeof aptosCoinStoreStructTag | string,
}

export interface CoinInfoData {
  decimals: number;
  name: string;
  symbol: string;
}

export type CoinInfoResourceData = CoinInfoData & {
  supply: any,
};

export interface CoinInfoResource {
  data: CoinInfoResourceData,
  type: string,
}

export interface StakePoolResource {
  data: any,
  type: typeof aptosStakePoolStructTag,
}

export type Resource = CoinStoreResource | StakePoolResource;
