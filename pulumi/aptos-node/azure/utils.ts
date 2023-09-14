import { glob } from "glob";

import * as std from "@pulumi/std";

export async function getSha1ForDirs(path: string) {
  const results = await glob(`${path}/**`, { nodir: true, stat: true, withFileTypes: true });
  const timeSortedFiles = results.sort((a, b) => a.mtimeMs! - b.mtimeMs!).map((path) => path.fullpath());
  return timeSortedFiles.map(
    (f) =>
      std.filesha1Output({
        input: f,
      }).result,
  );
}

export interface AptosLoadBalancerIngress {
  ip: string;
  hostname: string;
  port: number;
}