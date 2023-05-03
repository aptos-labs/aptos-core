// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

const ffi = require("ffi-napi");

/**
 * Aptos CLI wrapper - Allows to run the aptos CLI
 * synchronously and asynchronously from Typescript
 */
export class AptosCLI {
  private lib: any;

  constructor(aptos_dylib_path: string = "../../../../../target/release/libaptos") {
    this.lib = ffi.Library(aptos_dylib_path, {
      run_aptos_sync: ["char *", ["string"]], // run the aptos CLI synchronously
      run_aptos_async: ["char *", ["string"]], // run the aptos CLI asynchronously
      free_cstring: ["void", ["char *"]], // free the return pointer memory allocated by the aptos CLI
    });
  }

  public runSync(args: string[]): string {
    const result = this.lib.run_aptos_sync(args.join(" "));
    const output = result.readCString();
    this.lib.free_cstring(result);
    return output;
  }

  public async runAsync(args: string[]): Promise<string> {
    const result = this.lib.run_aptos_async(args.join(" "));
    const output = await new Promise<string>((resolve, reject) => {
      result((error: any, result: any) => {
        if (error) {
          reject(error);
        } else {
          const output = result.readCString();
          this.lib.free_cstring(result);
          resolve(output);
        }
      });
    });
    return output;
  }

  /**
   * get aptos info
   */
  public getAptosInfo(): string {
    return this.runSync(["aptos", "info"]);
  }
}
