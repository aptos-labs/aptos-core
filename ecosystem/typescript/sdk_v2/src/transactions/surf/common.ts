// eslint-disable-next-line @typescript-eslint/no-unused-vars
export type UnknownStruct<T extends string> = object;

// Remove all `signer` and `&signer` from argument list because the Move VM injects those arguments. Clients do not
// need to care about those args. `signer` and `&signer` are required be in the front of the argument list.
export type OmitSigner<T extends readonly string[]> = T extends readonly [
  "&signer" | "signer",
  ...infer Rest,
]
  ? Rest
  : T;

// Remove the inner type of a string.
// For example, `0x1::object::Object<u8>` -> `0x1::object::Object`.
export type OmitInner<T extends string> =
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  T extends `${infer TOuter}<${infer TInner}>` ? `${TOuter}` : T;
