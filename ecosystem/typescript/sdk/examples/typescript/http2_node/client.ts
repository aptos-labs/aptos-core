import { HexString, MaybeHexString } from "../../../dist";

const { connect } = require("http2");

export class Client {
  private client: any = connect("https://fullnode.testnet.aptoslabs.com");
  private faucet: any = connect("https://faucet.testnet.aptoslabs.com");

  get(url: string): Promise<any> {
    return new Promise((resolve, reject) => {
      const request = this.client.request({ ":path": "/v1/" + url, "content-type": "application/json" });
      request.on("response", (headers: any) => {
        //console.log("headers", headers);
      });
      let chunks = "";
      request.on("data", (chunk: any) => {
        chunks += chunk;
      });
      request.on("end", () => {
        //const data = JSON.parse(chunks);
        resolve(chunks);
      });
      request.on("error", (error: any) => {
        reject(error);
      });
      //request.end();
    });
  }

  getAccount(address: string): Promise<any> {
    return this.get(`accounts/${address}`);
  }

  submitTransaction(transaction: Uint8Array): Promise<any> {
    return new Promise((resolve, reject) => {
      const req = this.client.request({
        ":method": "POST",
        ":path": "/v1/transactions",
        "content-type": "application/x.aptos.signed_transaction+bcs",
        "content-length": Buffer.byteLength(transaction),
      });

      let data = "";

      req.on("response", (headers: any, flags: any) => {
        //console.log("headers", headers);
      });

      req.on("data", (chunk: any) => {
        data += chunk;
      });

      req.on("end", () => {
        //console.log(`Request completed`, data);
        resolve(data);
      });

      req.on("error", (err: any) => {
        console.error(`Request failed: ${err}`);
        reject(err);
      });

      req.write(transaction);
      req.end();
    });
  }

  fundAccount(address: MaybeHexString, amount: number): Promise<any> {
    return new Promise((resolve, reject) => {
      const body = `address=${HexString.ensure(address).noPrefix()}&amount=${amount}`;

      const req = this.faucet.request({
        ":method": "POST",
        ":path": "/mint?" + body,
      });

      let data = "";

      req.on("response", (headers: any, flags: any) => {
        //console.log("headers", headers);
      });

      req.on("data", (chunk: any) => {
        data += chunk;
      });

      req.on("end", () => {
        //console.log(`Request completed`, data);
        //console.log("fund", data);
        resolve(data);
      });

      req.on("error", (err: any) => {
        console.error(`Request failed: ${err}`);
        reject(err);
      });
      req.end();
    });
  }
}
