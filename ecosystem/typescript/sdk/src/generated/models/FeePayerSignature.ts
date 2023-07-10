/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

import type { AccountSignature } from './AccountSignature';
import type { Address } from './Address';

/**
 * Fee payer signature for fee payer transactions
 *
 * This allows you to have transactions across multiple accounts and with a fee payer
 */
export type FeePayerSignature = {
    sender: AccountSignature;
    /**
     * The other involved parties' addresses
     */
    secondary_signer_addresses: Array<Address>;
    /**
     * The associated signatures, in the same order as the secondary addresses
     */
    secondary_signers: Array<AccountSignature>;
    fee_payer_address: Address;
    fee_payer_signer: AccountSignature;
};

