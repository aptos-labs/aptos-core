// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { Types } from 'aptos';

/**
 * The REST API exposes the event's originating transaction version,
 * but the type in the Aptos SDK has not been updated yet
 */
export type EventWithVersion = Types.Event & { version: string };

/**
 * Preprocessed version of an Event. Mostly using numbers instead of strings
 */
interface BaseEvent {
  guid: {
    address: string,
    creationNumber: number,
  },
  sequenceNumber: number,
  version: number,
}

export type WithdrawEvent = BaseEvent & {
  data: { amount: string },
  type: '0x1::coin::WithdrawEvent',
};

export type DepositEvent = BaseEvent & {
  data: { amount: string },
  type: '0x1::coin::DepositEvent',
};

export type GenericEvent = BaseEvent & {
  data: any,
  type: Types.MoveType,
};

export type Event = WithdrawEvent | DepositEvent | GenericEvent;
