// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

/* eslint-disable no-bitwise */

// region Enumerations

/**
 * The upper byte of an abort code identifies the category, which is shared across move modules
 * See https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/move-stdlib/sources/error.move
 */
export enum MoveAbortCategory {
  InvalidArgument = 0x1,
  OutOfRange,
  InvalidState,
  Unauthenticated,
  PermissionDenied,
  NotFound,
  Aborted,
  AlreadyExists,
  ResourceExhausted,
  Cancelled,
  Internal,
  NotImplemented,
  Unavailable,
}

/**
 * The lower two bytes of an abort code identify the module-specific abort reason
 */
type MoveAbortReason = AccountErrorReason | CoinErrorReason | TransactionValidationErrorReason;

// See https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-framework/sources/account.move
export enum AccountErrorReason {
  AccountAlreadyExists = 1,
  AccountDoesNotExist,
  SequenceNumberTooBig,
  MalformedAuthenticationKey,
  CannotReservedAddress,
  OutOfGas,
  WrongCurrentPublicKey,
  InvalidProofOfKnowledge,
  NoCapability,
  InvalidAcceptRotationCapability,
  NoValidFrameworkReservedAddress,
}

// See https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-framework/sources/coin.move
export enum CoinErrorReason {
  CoinInfoAddressMismatch = 1,
  CoinInfoAlreadyPublished,
  CoinInfoNotPublished,
  CoinStoreAlreadyPublished,
  CoinStoreNotPublished,
  InsufficientBalance,
  DestructionOfNonZeroToken,
  TotalSupplyOverflow,
  InvalidCoinAmount,
  Frozen,
  CoinSupplyUpgradeNotSupported,
}

// See https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-framework/sources/transaction_validation.move
export enum TransactionValidationErrorReason {
  OutOfGas = 6,
  InvalidAccountAuthKey = 1001,
  SequenceNumberTooOld,
  SequenceNumberTooNew,
  AccountDoesNotExist,
  CantPayGasDeposit,
  TransactionExpired,
  BadChainId,
  SequenceNumberTooBig,
  SecondaryKeysAddressesCountMismatch,
}

// endregion

/**
 * Mapping to retrieve module-specific reasons from the location
 */
const moduleReasonsMap = {
  '0x1::account': AccountErrorReason,
  '0x1::coin': CoinErrorReason,
  '0x1::transaction_validation': TransactionValidationErrorReason,
};

export type MoveAbortLocation = keyof typeof moduleReasonsMap;

/**
 * Move abort message pattern is a little messy, hopefully we can standardize it in the future
 * See `explain_vm_status` at https://github.com/aptos-labs/aptos-core/blob/main/api/types/src/convert.rs
 */
const moveAbortPattern = /^Move abort(?: in (0x[\da-f]+::\S+):|: code) 0x([\da-f]+)$/;
const moveAbortPatternWithDescr = /^Move abort in (0x[\da-f]+::\S+): ([^(]+)\(0x([\da-f]+)\): (.*)$/;

/**
 * Parse raw abort details from the VM status of a transaction
 * @param vmStatus status of a transaction
 */
function parseMoveAbortRawDetails(vmStatus: string) {
  let match: RegExpMatchArray | null;

  match = vmStatus.match(moveAbortPatternWithDescr);
  if (match) {
    const location = match[1] as MoveAbortLocation;
    const reasonName = match[2];
    const code = parseInt(match[3], 16);
    const reasonDescr = match[4];
    return {
      code,
      location,
      reasonDescr,
      reasonName,
    };
  }

  match = vmStatus.match(moveAbortPattern);
  if (match) {
    const location = match[1] as MoveAbortLocation | undefined;
    const code = parseInt(match[2], 16);
    return { code, location };
  }

  return undefined;
}

/**
 * Parse abort details from the VM status of a transaction
 * @param vmStatus status of a transaction
 */
export function parseMoveAbortDetails(vmStatus: string) {
  const details = parseMoveAbortRawDetails(vmStatus);
  if (!details) {
    return undefined;
  }

  const { code, location } = details;
  const category = (code & 0xff0000) >> 16 as MoveAbortCategory;
  const reason = (code & 0xffff) as MoveAbortReason;
  let { reasonDescr } = details;

  // Retrieve user-friendly abort description if available
  if (reasonDescr === undefined && location !== undefined) {
    const categoryTxt = MoveAbortCategory[category];
    const moduleReasons = moduleReasonsMap[location];
    const reasonTxt = moduleReasons !== undefined
      ? moduleReasons[reason]
      : undefined;

    if (categoryTxt !== undefined && reasonTxt !== undefined) {
      reasonDescr = `${categoryTxt}: ${reasonTxt}`;
    }
  }

  // Show details as fallback
  if (reasonDescr === undefined) {
    const codeHex = code.toString(16);
    const locationSuffix = location ? ` in ${location}` : undefined;
    reasonDescr = `Move abort 0x${codeHex}${locationSuffix}`;
  }

  return {
    category,
    code,
    location,
    reason,
    reasonDescr,
  };
}
