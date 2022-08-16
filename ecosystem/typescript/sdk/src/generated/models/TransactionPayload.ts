/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

import type { TransactionPayload_ModuleBundlePayload } from './TransactionPayload_ModuleBundlePayload';
import type { TransactionPayload_EntryFunctionPayload } from './TransactionPayload_EntryFunctionPayload';
import type { TransactionPayload_ScriptPayload } from './TransactionPayload_ScriptPayload';
import type { TransactionPayload_WriteSetPayload } from './TransactionPayload_WriteSetPayload';

export type TransactionPayload = (TransactionPayload_EntryFunctionPayload | TransactionPayload_ScriptPayload | TransactionPayload_ModuleBundlePayload | TransactionPayload_WriteSetPayload);

