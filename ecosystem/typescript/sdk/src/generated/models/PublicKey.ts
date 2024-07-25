/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

import type { PublicKey_Ed25519 } from './PublicKey_Ed25519';
import type { PublicKey_Keyless } from './PublicKey_Keyless';
import type { PublicKey_Secp256k1Ecdsa } from './PublicKey_Secp256k1Ecdsa';
import type { PublicKey_Secp256r1Ecdsa } from './PublicKey_Secp256r1Ecdsa';

export type PublicKey = (PublicKey_Ed25519 | PublicKey_Secp256k1Ecdsa | PublicKey_Secp256r1Ecdsa | PublicKey_Keyless);

