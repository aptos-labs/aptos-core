/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

import type { WriteSetChange_DeleteModule } from './WriteSetChange_DeleteModule';
import type { WriteSetChange_DeleteResource } from './WriteSetChange_DeleteResource';
import type { WriteSetChange_DeleteTableItem } from './WriteSetChange_DeleteTableItem';
import type { WriteSetChange_WriteModule } from './WriteSetChange_WriteModule';
import type { WriteSetChange_WriteResource } from './WriteSetChange_WriteResource';
import type { WriteSetChange_WriteTableItem } from './WriteSetChange_WriteTableItem';

/**
 * A final state change of a transaction on a resource or module
 */
export type WriteSetChange = (WriteSetChange_DeleteModule | WriteSetChange_DeleteResource | WriteSetChange_DeleteTableItem | WriteSetChange_WriteModule | WriteSetChange_WriteResource | WriteSetChange_WriteTableItem);

