/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

import type { TransactionPayload_EntryFunctionPayload } from './TransactionPayload_EntryFunctionPayload';
import type { TransactionPayload_ModuleBundlePayload } from './TransactionPayload_ModuleBundlePayload';
import type { TransactionPayload_MultisigPayload } from './TransactionPayload_MultisigPayload';
import type { TransactionPayload_ScriptPayload } from './TransactionPayload_ScriptPayload';

/**
 * An enum of the possible transaction payloads
 */
export type TransactionPayload = (TransactionPayload_EntryFunctionPayload | TransactionPayload_ScriptPayload | TransactionPayload_ModuleBundlePayload | TransactionPayload_MultisigPayload);

