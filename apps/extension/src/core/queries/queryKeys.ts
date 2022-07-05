import { accountQueryKeys } from './account';
import { collectiblesQueryKeys } from './collectibles';
import { networkQueryKeys } from './network';
import { transactionQueryKeys } from './transaction';

export const queryKeys = {
  ...collectiblesQueryKeys,
  ...accountQueryKeys,
  ...networkQueryKeys,
  ...transactionQueryKeys,
};

export default queryKeys;
