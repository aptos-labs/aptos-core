// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

/**
 * This error is used to explain why parsing failed.
 */
export class ParsingError<T> extends Error {
  /**
   * This provides a programmatic way to access why parsing failed. Downstream devs
   * might want to use this to build their own error messages if the default error
   * messages are not suitable for their use case. This should be an enum.
   */
  public invalidReason: T;

  constructor(message: string, invalidReason: T) {
    super(message);
    this.invalidReason = invalidReason;
  }
}

/**
 * Whereas ParsingError is thrown when parsing fails, e.g. in a fromString function,
 * this type is returned from "defensive" functions like isValid.
 */
export type ParsingResult<T> = {
  /**
   * True if valid, false otherwise.
   */
  valid: boolean;

  /*
   * If valid is false, this will be a code explaining why parsing failed.
   */
  invalidReason?: T;

  /*
   * If valid is false, this will be a string explaining why parsing failed.
   */
  invalidReasonMessage?: string;
};
