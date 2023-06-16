import { AxiosResponse } from "axios";
import { AptosRequest, aptosRequest } from "./core";

export type GetRequestOptions<Req> = Omit<AptosRequest<Req>, "body" | "method">;

/**
 * Main function to do a Get request
 *
 * @param options GetRequestOptions
 * @returns
 */
export async function get<Req, Res>(options: GetRequestOptions<Req>): Promise<AxiosResponse<Res, any>> {
  const response: AxiosResponse = await aptosRequest<Req, Res>({ ...options, method: "GET" });
  return response;
}
