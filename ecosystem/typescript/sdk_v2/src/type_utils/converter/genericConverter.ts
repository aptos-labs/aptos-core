/**
 * Convert an array of generic arguments from Move type to TypeScript type.
 */
export type ConvertGenerics<T extends readonly any[]> = T extends readonly [
  any,
  ...infer TRest,
]
  ? [string, ...ConvertGenerics<TRest>]
  : [];
