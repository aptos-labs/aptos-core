import { connect } from "http2";

interface RequestOptions {
  method: "GET" | "POST";
  path: string;
  headers?: { [key: string]: any };
  body?: any;
}

export class Http2Client {
  private endpoint: string;
  private requestCounter = 0; // Counter for pending requests
  private session: any;

  constructor(endpoint: string) {
    this.endpoint = endpoint;
  }

  incrementCounter() {
    this.requestCounter++;
  }

  decrementCounter() {
    this.requestCounter--;
    if (this.requestCounter === 0) {
      // Close the session when all requests are completed
      this.session.close();
    }
  }

  request(options: RequestOptions): Promise<string> {
    return new Promise((resolve, reject) => {
      this.incrementCounter();

      this.session = connect(this.endpoint);

      const req = this.session.request({
        ":path": options.path,
        ":method": options.method,
        ...options.headers,
      });

      if (options.body) {
        // Write the query to the request body
        req.write(options.body);
      }

      // Handle the response
      req.on("response", (headers: any) => {
        const chunks: any = [];

        req.on("data", (chunk: any) => {
          chunks.push(chunk);
        });

        req.on("end", () => {
          const responseBody = Buffer.concat(chunks).toString();
          const response: any = {
            data: responseBody,
            _headers: headers,
          };
          this.decrementCounter();
          resolve(response);
        });
      });

      // Handle any errors that occur during the request
      req.on("error", (error: any) => {
        reject(error);
      });

      // End the request
      req.end();
    });
  }

  get(path: string, headers?: { [key: string]: any }): Promise<string> {
    const options: RequestOptions = {
      method: "GET",
      path,
      headers,
    };

    return this.request(options);
  }

  post(path: string, body: any, headers?: { [key: string]: any }): Promise<string> {
    const options: RequestOptions = {
      method: "POST",
      path,
      body,
      headers,
    };

    return this.request(options);
  }
}
