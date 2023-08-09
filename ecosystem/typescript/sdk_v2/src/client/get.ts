import { AptosRequest, AptosResponse } from "./types";
import { aptosRequest } from "./core";

export type GetRequestOptions = Omit<AptosRequest, "body" | "method">;

/**
 * Main function to do a Get request
 *
 * @param options GetRequestOptions
 * @returns
 */
export async function get<Req, Res>(options: GetRequestOptions): Promise<AptosResponse<Req, Res>> {
  const response: AptosResponse<Req, Res> = await aptosRequest<Req, Res>({ ...options, method: "GET" });
  return response;
}
