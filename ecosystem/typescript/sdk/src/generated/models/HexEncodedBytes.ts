/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

/**
 * All bytes (Vec<u8>) data is represented as hex-encoded string prefixed with `0x` and fulfilled with
 * two hex digits per byte.
 *
 * Unlike the `Address` type, HexEncodedBytes will not trim any zeros.
 *
 */
export type HexEncodedBytes = string;
