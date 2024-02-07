import { AptosClientRequest, AptosClientResponse } from "./types";

export default async function aptosClient<Res>(options: AptosClientRequest): Promise<AptosClientResponse<Res>> {
  const { params, method, url, headers, body } = options;

  // Constructing the query string for POST requests
  let queryString = '';
  if (params && method.toUpperCase() === 'POST') {
    queryString = `?address=${params.address}&amount=${params.amount}`
  }

  // Setting up the Request Configuration for fetch
  const requestConfig: RequestInit = {
    method: method,
    headers: headers,
  };
  if (body) {
    if (body instanceof Uint8Array) {
      requestConfig.body = Buffer.from(body);
    } else {
      requestConfig.body = Buffer.from(JSON.stringify(body));
    }
  }

  try {
    const response = await fetch(url + queryString, requestConfig);
    const responseData = await response.json();

    return {
      status: response.status,
      statusText: response.statusText,
      data: responseData as Res,
      headers: response.headers,
      request: requestConfig,
      response: response,
    };
  } catch (error) {
    if (error instanceof Response) {
      const errorResponse = await error.json();
      return {
        status: error.status,
        statusText: error.statusText,
        data: errorResponse,
        headers: error.headers,
        request: requestConfig,
        response: error,
      };
    }
    throw error;
  }
}
