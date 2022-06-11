import { accountQueryKeys } from './account';
import { collectiblesQueryKeys } from './collectibles';
import { networkQueryKeys } from './network';

export const queryKeys = {
  ...collectiblesQueryKeys,
  ...accountQueryKeys,
  ...networkQueryKeys,
};

export default queryKeys;
