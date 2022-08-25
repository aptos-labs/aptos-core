/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

import type { AccountSignature } from './AccountSignature.js';
import type { Address } from './Address.js';

export type MultiAgentSignature = {
    sender: AccountSignature;
    secondary_signer_addresses: Array<Address>;
    secondary_signers: Array<AccountSignature>;
};

