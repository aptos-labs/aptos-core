/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

import type { TransactionPayload_ModuleBundlePayload } from './TransactionPayload_ModuleBundlePayload';
import type { TransactionPayload_ScriptFunctionPayload } from './TransactionPayload_ScriptFunctionPayload';
import type { TransactionPayload_ScriptPayload } from './TransactionPayload_ScriptPayload';
import type { TransactionPayload_WriteSetPayload } from './TransactionPayload_WriteSetPayload';

export type TransactionPayload = (TransactionPayload_ScriptFunctionPayload | TransactionPayload_ScriptPayload | TransactionPayload_ModuleBundlePayload | TransactionPayload_WriteSetPayload);

