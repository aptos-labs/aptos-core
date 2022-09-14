export * from './build';
export * from './submit';
export * from './utils';

// TODO: make this parameters configurable by the user
const defaultGasMultiplier = 2;

export function maxGasFeeFromEstimated(
  estimatedGasFee: number,
  multiplier = defaultGasMultiplier,
) {
  return estimatedGasFee * multiplier;
}
