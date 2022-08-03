/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

import type { AccountSignature_Ed25519Signature } from './AccountSignature_Ed25519Signature';
import type { AccountSignature_MultiEd25519Signature } from './AccountSignature_MultiEd25519Signature';

export type AccountSignature = (AccountSignature_Ed25519Signature | AccountSignature_MultiEd25519Signature);

