/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

import type { TransactionPayload_ModuleBundlePayload } from './TransactionPayload_ModuleBundlePayload';
import type { TransactionPayload_ScriptFunctionPayload } from './TransactionPayload_ScriptFunctionPayload';
import type { TransactionPayload_ScriptPayload } from './TransactionPayload_ScriptPayload';

export type TransactionPayload = (TransactionPayload_ScriptFunctionPayload | TransactionPayload_ScriptPayload | TransactionPayload_ModuleBundlePayload);

