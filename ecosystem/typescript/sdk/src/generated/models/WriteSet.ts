/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

import type { WriteSet_DirectWriteSet } from './WriteSet_DirectWriteSet.js';
import type { WriteSet_ScriptWriteSet } from './WriteSet_ScriptWriteSet.js';

export type WriteSet = (WriteSet_ScriptWriteSet | WriteSet_DirectWriteSet);

