/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

import type { AccountSignature } from './AccountSignature';
import type { Address } from './Address';

/**
 * Multi agent signature for multi agent transactions
 *
 * This allows you to have transactions across multiple accounts
 */
export type MultiAgentSignature = {
    sender: AccountSignature;
    /**
     * The other involved parties' addresses
     */
    secondary_signer_addresses: Array<Address>;
    /**
     * The associated signatures, in the same order as the secondary addresses
     */
    secondary_signers: Array<AccountSignature>;
};

