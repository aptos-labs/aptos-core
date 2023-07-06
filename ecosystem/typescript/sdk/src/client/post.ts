import { aptosRequest } from "./core";
import { AptosRequest, AptosResponse } from "./types";

export type PostRequestOptions = Omit<AptosRequest, "method">;

/**
 * Main function to do a Post request
 *
 * @param options PostRequestOptions
 * @returns
 */
export async function post<Req, Res>(options: PostRequestOptions): Promise<AptosResponse<Req, Res>> {
  const response: AptosResponse<Req, Res> = await aptosRequest<Req, Res>({ ...options, method: "POST" });
  return response;
}
