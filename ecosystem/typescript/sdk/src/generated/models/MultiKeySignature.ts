/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

import type { IndexedSignature } from './IndexedSignature';
import type { PublicKey } from './PublicKey';

/**
 * A multi key signature
 */
export type MultiKeySignature = {
    public_keys: Array<PublicKey>;
    signatures: Array<IndexedSignature>;
    signatures_required: number;
};

