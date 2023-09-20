import { exec } from "child_process";
import http, { IncomingMessage } from "http";

export class AptosNode {

  static MAXIMUM_WAIT_TIME_SEC = 30;

  async start() {

    let nodeIsUp = await this.checkIfProcessIsUp();
    if(nodeIsUp)
      return;

    const cliCommand =
      "npx aptos node run-local-testnet --force-restart --assume-yes --with-faucet";

    const childProcess = exec(cliCommand);

    childProcess.stdout?.on("data", (data: any) => {
      console.log(`CLI Process Output: ${data}`);
    });

    childProcess.stderr?.on("data", (data: any) => {
      console.error(`CLI Process Error: ${data}`);
    });

    childProcess.on("close", (code: any) => {
      console.log(`CLI Process Exited with Code: ${code}`);
    });
  }

  async waitUntilProcessIsUp() {
    let operational = await this.checkIfProcessIsUp();
    let start = Date.now() / 1000;
    let last = start;

    while (!operational && start + AptosNode.MAXIMUM_WAIT_TIME_SEC > last) {
      await this.sleep(1000);
      operational = await this.checkIfProcessIsUp();
      last = Date.now() / 1000;
    }
    console.log("local node is up");
  }

  async checkIfProcessIsUp(): Promise<boolean> {
    return new Promise<boolean>((resolve, reject) => {
      const options = {
        hostname: "127.0.0.1",
        port: 8081,
        path: "/",
        method: "GET",
      };

      const req = http.request(options, (res: IncomingMessage) => {
        if (res.statusCode === 200) {
          resolve(true);
        } else {
          resolve(false);
        }
      });

      req.on("error", (error: any) => {
        resolve(false);
      });

      req.end();
    });
  }

  private async sleep(timeMs: number): Promise<null> {
    return new Promise((resolve) => {
      setTimeout(resolve, timeMs);
    });
  }
}
