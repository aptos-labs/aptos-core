/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

/**
 * A string containing a 128-bit unsigned integer.
 *
 * We represent u128 values as a string to ensure compatibility with languages such
 * as JavaScript that do not parse u64s in JSON natively.
 *
 */
export type U128 = string;
