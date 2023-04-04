/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

import type { Transaction_BlockMetadataTransaction } from './Transaction_BlockMetadataTransaction';
import type { Transaction_GenesisTransaction } from './Transaction_GenesisTransaction';
import type { Transaction_PendingTransaction } from './Transaction_PendingTransaction';
import type { Transaction_StateCheckpointTransaction } from './Transaction_StateCheckpointTransaction';
import type { Transaction_UserTransaction } from './Transaction_UserTransaction';

/**
 * Enum of the different types of transactions in Aptos
 */
export type Transaction = (Transaction_PendingTransaction | Transaction_UserTransaction | Transaction_GenesisTransaction | Transaction_BlockMetadataTransaction | Transaction_StateCheckpointTransaction);

