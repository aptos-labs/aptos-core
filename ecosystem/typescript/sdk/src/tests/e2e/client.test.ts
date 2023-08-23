import { AptosApiError, aptosRequest } from "../../client";
import { VERSION } from "../../version";
import { getTransaction, longTestTimeout, NODE_URL } from "../unit/test_helper.test";

test(
  "call should include x-aptos-client header",
  async () => {
    try {
      const response = await aptosRequest({
        url: `${NODE_URL}`,
        method: "GET",
        endpoint: "accounts/0x1",
        body: null,
        originMethod: "test x-aptos-client header",
      });
      expect(response.config.headers).toHaveProperty("x-aptos-client", `aptos-ts-sdk/${VERSION}`);
    } catch (error: any) {
      // should not get here
      expect(true).toBe(false);
    }
  },
  longTestTimeout,
);

test(
  "call should include all expected headers",
  async () => {
    const bcsTxn = await getTransaction();
    try {
      const response = await aptosRequest({
        url: `${NODE_URL}`,
        method: "POST",
        endpoint: "transactions",
        body: bcsTxn,
        originMethod: "test request includes all headers",
        contentType: "application/x.aptos.signed_transaction+bcs",
        overrides: { HEADERS: { my: "header" } },
      });
      expect(response.config.headers).toHaveProperty("x-aptos-client", `aptos-ts-sdk/${VERSION}`);
      expect(response.config.headers).toHaveProperty("my", "header");
      expect(response.config.headers).toHaveProperty("content-type", "application/x.aptos.signed_transaction+bcs");
    } catch (error: any) {
      // should not get here
      expect(true).toBe(false);
    }
  },
  longTestTimeout,
);

test(
  "when token is set",
  async () => {
    try {
      const response = await aptosRequest({
        url: `${NODE_URL}`,
        method: "GET",
        endpoint: "accounts/0x1",
        body: null,
        originMethod: "test 200 status",
        overrides: { TOKEN: "my-token" },
      });
      expect(response.config.headers).toHaveProperty("authorization", "Bearer my-token");
    } catch (error: any) {
      // should not get here
      expect(true).toBe(false);
    }
  },
  longTestTimeout,
);

test(
  "when token is not set",
  async () => {
    try {
      const response = await aptosRequest({
        url: `${NODE_URL}`,
        method: "GET",
        endpoint: "accounts/0x1",
        body: null,
        originMethod: "test 200 status",
      });
      expect(response.config.headers).not.toHaveProperty("authorization", "Bearer my-token");
    } catch (error: any) {
      // should not get here
      expect(true).toBe(false);
    }
  },
  longTestTimeout,
);

test(
  "when server returns 400 status code",
  async () => {
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
      expect(error.data).toEqual({
        message: 'failed to parse path `txn_hash`: failed to parse "string(HashValue)": unable to parse HashValue',
        error_code: "web_framework_error",
        vm_error_code: null,
      });
      expect(error.request).toEqual({
        url: `${NODE_URL}`,
        method: "GET",
        originMethod: "test 400 status",
        endpoint: "transactions/by_hash/0x123",
        body: null,
      });
    }
  },
  longTestTimeout,
);

test(
  "when server returns 200 status code",
  async () => {
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
  },
  longTestTimeout,
);

test(
  "when server returns 404 status code",
  async () => {
    try {
      await aptosRequest({
        url: `${NODE_URL}`,
        method: "GET",
        endpoint: "transactions/by_hash/0x23851af73879128b541bafad4b49d0b6f1ac0d49ed2400632d247135fbca7bea",
        body: null,
        originMethod: "test 404 status",
      });
    } catch (error: any) {
      expect(error).toBeInstanceOf(AptosApiError);
      expect(error.url).toBe(
        `${NODE_URL}/transactions/by_hash/0x23851af73879128b541bafad4b49d0b6f1ac0d49ed2400632d247135fbca7bea`,
      );
      expect(error.status).toBe(404);
      expect(error.statusText).toBe("Not Found");
      expect(error.data).toEqual({
        message:
          "Transaction not found by Transaction hash(0x23851af73879128b541bafad4b49d0b6f1ac0d49ed2400632d247135fbca7bea)",
        error_code: "transaction_not_found",
        vm_error_code: null,
      });
      expect(error.request).toEqual({
        url: `${NODE_URL}`,
        method: "GET",
        originMethod: "test 404 status",
        endpoint: "transactions/by_hash/0x23851af73879128b541bafad4b49d0b6f1ac0d49ed2400632d247135fbca7bea",
        body: null,
      });
    }
  },
  longTestTimeout,
);

test(
  "when server returns transaction submission error",
  async () => {
    try {
      await aptosRequest({
        url: `${NODE_URL}`,
        method: "POST",
        endpoint: "transactions",
        body: new Uint8Array([1, 2, 3]),
        originMethod: "test transaction submission error",
        contentType: "application/x.aptos.signed_transaction+bcs",
      });
    } catch (error: any) {
      expect(error).toBeInstanceOf(AptosApiError);
      expect(error.url).toBe(`${NODE_URL}/transactions`);
      expect(error.status).toBe(400);
      expect(error.statusText).toBe("Bad Request");
      expect(error.data).toEqual({
        message: "Failed to deserialize input into SignedTransaction: unexpected end of input",
        error_code: "invalid_input",
        vm_error_code: null,
      });
      expect(error.request).toEqual({
        url: `${NODE_URL}`,
        method: "POST",
        originMethod: "test transaction submission error",
        endpoint: "transactions",
        body: new Uint8Array([1, 2, 3]),
        contentType: "application/x.aptos.signed_transaction+bcs",
      });
    }
  },
  longTestTimeout,
);
