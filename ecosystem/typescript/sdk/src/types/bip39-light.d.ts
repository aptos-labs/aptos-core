declare module 'bip39-light' {
  export function entropyToMnemonic(
    entropy: Buffer | string,
    wordlist?: string[]
  ): string;
  export function generateMnemonic(
    strength?: number,
    rng?: (size: number) => Buffer,
    wordlist?: string[]
  ): string;
  export function mnemonicToSeedHex(mnemonic: string, password?: string): string;
  export function validateMnemonic(mnemonic: string): boolean;
}
