/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

import type { Address } from './Address';
import type { MultisigTransactionPayload } from './MultisigTransactionPayload';

/**
 * A multisig transaction that allows an owner of a multisig account to execute a pre-approved
 * transaction as the multisig account.
 */
export type MultisigPayload = {
    multisig_address: Address;
    transaction_payload?: MultisigTransactionPayload;
};

