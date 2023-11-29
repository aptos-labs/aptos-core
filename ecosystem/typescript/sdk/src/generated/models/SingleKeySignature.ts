/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

import type { PublicKey } from './PublicKey';
import type { Signature } from './Signature';

/**
 * A single key signature
 */
export type SingleKeySignature = {
    public_key: PublicKey;
    signature: Signature;
};

