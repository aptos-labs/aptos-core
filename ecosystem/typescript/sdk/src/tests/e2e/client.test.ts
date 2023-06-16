import { AptosApiError, aptosRequest } from "../../client";
import { NODE_URL } from "../unit/test_helper.test";

test("when token is set", async () => {
  try {
    const response = await aptosRequest({
      url: `${NODE_URL}`,
      method: "GET",
      endpoint: "accounts/0x1",
      body: null,
      originMethod: "test 200 status",
      overrides: { token: "my-token" },
    });
    expect(response.config.headers).toHaveProperty("Authorization", "Bearer my-token");
  } catch (error: any) {
    // should not get here
    expect(true).toBe(false);
  }
});

test("when token is not set", async () => {
  try {
    const response = await aptosRequest({
      url: `${NODE_URL}`,
      method: "GET",
      endpoint: "accounts/0x1",
      body: null,
      originMethod: "test 200 status",
    });
    expect(response.config.headers).not.toHaveProperty("Authorization", "Bearer my-token");
  } catch (error: any) {
    // should not get here
    expect(true).toBe(false);
  }
});

test("when server returns 400 status code", async () => {
  try {
    await aptosRequest({
      url: `${NODE_URL}`,
      method: "GET",
      endpoint: "transactions/by_hash/0x123",
      body: null,
      originMethod: "test 400 status",
    });
  } catch (error: any) {
    expect(error).toBeInstanceOf(AptosApiError);
    expect(error.url).toBe(`${NODE_URL}/transactions/by_hash/0x123`);
    expect(error.status).toBe(400);
    expect(error.statusText).toBe("Bad Request");
    expect(error.body).toEqual({
      message: 'failed to parse path `txn_hash`: failed to parse "string(HashValue)": unable to parse HashValue',
      error_code: "web_framework_error",
      vm_error_code: null,
    });
    expect(error.request).toEqual({
      url: `${NODE_URL}/transactions/by_hash/0x123`,
      method: "GET",
      originMethod: "test 400 status",
    });
  }
});

test("when server returns 200 status code", async () => {
  try {
    const response = await aptosRequest({
      url: `${NODE_URL}`,
      method: "GET",
      endpoint: "accounts/0x1",
      body: null,
      originMethod: "test 200 status",
    });
    expect(response).toHaveProperty("data", {
      sequence_number: "0",
      authentication_key: "0x0000000000000000000000000000000000000000000000000000000000000001",
    });
  } catch (error: any) {
    // should not get here
    expect(true).toBe(false);
  }
});

test("when server returns 404 status code", async () => {
  try {
    const response = await aptosRequest({
      url: `${NODE_URL}`,
      method: "GET",
      endpoint: "transactions/by_hash/0x23851af73879128b541bafad4b49d0b6f1ac0d49ed2400632d247135fbca7bea",
      body: null,
      originMethod: "test 404 status",
    });
    expect(response).toHaveProperty("data", {
      sequence_number: "0",
      authentication_key: "0x0000000000000000000000000000000000000000000000000000000000000001",
    });
  } catch (error: any) {
    expect(error).toBeInstanceOf(AptosApiError);
    expect(error.url).toBe(
      `${NODE_URL}/transactions/by_hash/0x23851af73879128b541bafad4b49d0b6f1ac0d49ed2400632d247135fbca7bea`,
    );
    expect(error.status).toBe(404);
    expect(error.statusText).toBe("Not Found");
    expect(error.body).toEqual({
      message:
        "Transaction not found by Transaction hash(0x23851af73879128b541bafad4b49d0b6f1ac0d49ed2400632d247135fbca7bea)",
      error_code: "transaction_not_found",
      vm_error_code: null,
    });
    expect(error.request).toEqual({
      url: `${NODE_URL}/transactions/by_hash/0x23851af73879128b541bafad4b49d0b6f1ac0d49ed2400632d247135fbca7bea`,
      method: "GET",
      originMethod: "test 404 status",
    });
  }
});
