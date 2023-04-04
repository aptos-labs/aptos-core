/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

/**
 * Move module id is a string representation of Move module.
 *
 * Format: `{address}::{module name}`
 *
 * `address` should be hex-encoded 32 byte account address that is prefixed with `0x`.
 *
 * Module name is case-sensitive.
 *
 */
export type MoveModuleId = string;
