/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

import type { AccountSignature_Ed25519Signature } from './AccountSignature_Ed25519Signature';
import type { AccountSignature_MultiEd25519Signature } from './AccountSignature_MultiEd25519Signature';

/**
 * Account signature scheme
 *
 * The account signature scheme allows you to have two types of accounts:
 *
 * 1. A single Ed25519 key account, one private key
 * 2. A k-of-n multi-Ed25519 key account, multiple private keys, such that k-of-n must sign a transaction.
 */
export type AccountSignature = (AccountSignature_Ed25519Signature | AccountSignature_MultiEd25519Signature);

