/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

import type { TransactionPayload_EntryFunctionPayload } from './TransactionPayload_EntryFunctionPayload.js';
import type { TransactionPayload_ModuleBundlePayload } from './TransactionPayload_ModuleBundlePayload.js';
import type { TransactionPayload_ScriptPayload } from './TransactionPayload_ScriptPayload.js';

export type TransactionPayload = (TransactionPayload_EntryFunctionPayload | TransactionPayload_ScriptPayload | TransactionPayload_ModuleBundlePayload);

