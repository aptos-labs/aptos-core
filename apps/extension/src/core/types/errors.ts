// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

// eslint-disable-next-line max-classes-per-file
class ExtendableError extends Error {
  constructor(message: string) {
    super();
    this.message = message;
    if (typeof Error.captureStackTrace === 'function') {
      Error.captureStackTrace(this, this.constructor);
    } else {
      this.stack = (new Error(message)).stack;
    }
  }
}

export class DappError extends ExtendableError {
  code: number;

  constructor(code: number, message: string) {
    super(message);
    this.code = code;
  }
}

export const DappErrorType = Object.freeze({
  NO_ACCOUNTS: new DappError(4000, 'No accounts found'),
  TRANSACTION_FAILURE: new DappError(-30000, 'Transaction failed'),
  UNAUTHORIZED: new DappError(4100, 'The requested method and/or account has not been authorized by the user.'),
  UNSUPPORRTED: new DappError(4200, 'The provider does not support the requested method.'),
  USER_REJECTION: new DappError(4001, 'The user rejected the request'),
});
