/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

import type { Signature_Ed25519 } from './Signature_Ed25519';
import type { Signature_Keyless } from './Signature_Keyless';
import type { Signature_Secp256k1Ecdsa } from './Signature_Secp256k1Ecdsa';
import type { Signature_WebAuthn } from './Signature_WebAuthn';

export type Signature = (Signature_Ed25519 | Signature_Secp256k1Ecdsa | Signature_WebAuthn | Signature_Keyless);

