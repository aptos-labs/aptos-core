import { VERSION } from "../version";

export class ProviderUtil {
  static getUserAgent(): string {
    return `aptos-ts-sdk/${VERSION}`;
  }
}
