import { AxiosResponse } from "axios";
import { AptosRequest, aptosRequest } from "./core";

export type PostRequestOptions<Req> = Omit<AptosRequest<Req>, "method">;

/**
 * Main function to do a Post request
 *
 * @param options PostRequestOptions
 * @returns
 */
export async function post<Req, Res>(options: PostRequestOptions<Req>): Promise<AxiosResponse<Res, any>> {
  const response: AxiosResponse = await aptosRequest<Req, Res>({ ...options, method: "POST" });
  return response;
}
