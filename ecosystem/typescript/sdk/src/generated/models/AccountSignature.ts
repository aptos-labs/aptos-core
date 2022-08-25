/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

import type { AccountSignature_Ed25519Signature } from './AccountSignature_Ed25519Signature.js';
import type { AccountSignature_MultiEd25519Signature } from './AccountSignature_MultiEd25519Signature.js';

export type AccountSignature = (AccountSignature_Ed25519Signature | AccountSignature_MultiEd25519Signature);

