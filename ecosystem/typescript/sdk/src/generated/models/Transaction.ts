/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

import type { Transaction_BlockMetadataTransaction } from './Transaction_BlockMetadataTransaction.js';
import type { Transaction_GenesisTransaction } from './Transaction_GenesisTransaction.js';
import type { Transaction_PendingTransaction } from './Transaction_PendingTransaction.js';
import type { Transaction_StateCheckpointTransaction } from './Transaction_StateCheckpointTransaction.js';
import type { Transaction_UserTransaction } from './Transaction_UserTransaction.js';

export type Transaction = (Transaction_PendingTransaction | Transaction_UserTransaction | Transaction_GenesisTransaction | Transaction_BlockMetadataTransaction | Transaction_StateCheckpointTransaction);

