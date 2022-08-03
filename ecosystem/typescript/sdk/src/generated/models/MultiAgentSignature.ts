/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

import type { AccountSignature } from './AccountSignature';
import type { Address } from './Address';

export type MultiAgentSignature = {
    sender: AccountSignature;
    secondary_signer_addresses: Array<Address>;
    secondary_signers: Array<AccountSignature>;
};

