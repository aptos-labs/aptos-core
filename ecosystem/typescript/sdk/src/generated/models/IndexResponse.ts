/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

import type { RoleType } from './RoleType';
import type { U64 } from './U64';

/**
 * The struct holding all data returned to the client by the
 * index endpoint (i.e., GET "/").
 */
export type IndexResponse = {
    chain_id: number;
    epoch: U64;
    ledger_version: U64;
    oldest_ledger_version: U64;
    ledger_timestamp: U64;
    node_role: RoleType;
};

