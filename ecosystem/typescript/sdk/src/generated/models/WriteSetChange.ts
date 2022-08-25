/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

import type { WriteSetChange_DeleteModule } from './WriteSetChange_DeleteModule.js';
import type { WriteSetChange_DeleteResource } from './WriteSetChange_DeleteResource.js';
import type { WriteSetChange_DeleteTableItem } from './WriteSetChange_DeleteTableItem.js';
import type { WriteSetChange_WriteModule } from './WriteSetChange_WriteModule.js';
import type { WriteSetChange_WriteResource } from './WriteSetChange_WriteResource.js';
import type { WriteSetChange_WriteTableItem } from './WriteSetChange_WriteTableItem.js';

export type WriteSetChange = (WriteSetChange_DeleteModule | WriteSetChange_DeleteResource | WriteSetChange_DeleteTableItem | WriteSetChange_WriteModule | WriteSetChange_WriteResource | WriteSetChange_WriteTableItem);

