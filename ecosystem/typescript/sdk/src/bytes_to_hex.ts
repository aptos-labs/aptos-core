/**
 * Converts a {@link Uint8Array} to a hexadecimal string.
 * @param buffer The buffer to convert.
 * @returns
 */
export const bytesToHex = (buffer: Uint8Array): string =>
  [...buffer].map((x) => x.toString(16).padStart(2, "0")).join("");

/**
 * Checks if a string is a valid string of hex characters.
 * @param maybeHex
 * @returns
 */
export const isValidHexString = (maybeHex: string) => !/[^a-fA-F0-9]/u.test(maybeHex);

/**
 * Zero pads a hex string to make it a byte string.
 * @param hex
 * @returns
 */
export const zeroPadHex = (hex: string): string => (hex.length % 2 === 1 ? `0${hex}` : hex);

/**
 * Converts a hexadecimal string to a {@link Uint8Array}.
 *
 * @param buffer The buffer to convert.
 * @returns
 */
export const hexToBytes = (hex: string): Uint8Array => {
  if (hex.length === 0) {
    return new Uint8Array();
  }
  const hexNormalized = zeroPadHex(hex);
  if (!isValidHexString(hexNormalized)) {
    throw new Error(`Invalid hex string: ${hex}`);
  }
  return Uint8Array.from((hexNormalized.match(/.{1,2}/g) ?? []).map((byte) => parseInt(byte, 16)));
};
