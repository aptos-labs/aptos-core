import { AuthenticationKey } from "../../src/crypto/authentication_key";
import { PublicKey } from "../../src/crypto/ed25519";
import { MultiPublicKey } from "../../src/crypto/multi_ed25519";
import { ed25519, multiEd25519PkTestObject } from "./helper";

describe("AuthenticationKey", () => {
  it("should create an instance with save the hexinput correctly", () => {
    const authKey = new AuthenticationKey({ data: ed25519.authKey });
    expect(authKey).toBeInstanceOf(AuthenticationKey);
    expect(authKey.data.toString()).toEqual(ed25519.authKey);
  });

  it("should throw an error with invalid hex input length", () => {
    const invalidHexInput = "0123456789abcdef"; // Invalid length
    expect(() => new AuthenticationKey({ data: invalidHexInput })).toThrowError(
      "Authentication Key length should be 32",
    );
  });

  it("should create AuthenticationKey from PublicKey", () => {
    const publicKey = new PublicKey({ hexInput: ed25519.publicKey });
    const authKey = AuthenticationKey.fromPublicKey({ publicKey });
    expect(authKey).toBeInstanceOf(AuthenticationKey);
    expect(authKey.data.toString()).toEqual(ed25519.authKey);
  });

  it("should create AuthenticationKey from MultiPublicKey", () => {
    // create the MultiPublicKey
    let edPksArray = [];
    for (let i = 0; i < multiEd25519PkTestObject.public_keys.length; i++) {
      edPksArray.push(new PublicKey({ hexInput: multiEd25519PkTestObject.public_keys[i] }));
    }

    const pubKeyMultiSig = new MultiPublicKey({
      publicKeys: edPksArray,
      threshold: multiEd25519PkTestObject.threshold,
    });

    const authKey = AuthenticationKey.fromMultiPublicKey({ multiPublicKey: pubKeyMultiSig });
    expect(authKey).toBeInstanceOf(AuthenticationKey);
    expect(authKey.data.toString()).toEqual("0xa81cfac3df59920593ff417b45fc347ead3d88f8e25112c0488d34d7c9eb20af");
  });

  it("should derive an AccountAddress from AuthenticationKey with same string", () => {
    const authKey = new AuthenticationKey({ data: ed25519.authKey });
    const accountAddress = authKey.derivedAddress();
    expect(accountAddress.toString()).toEqual(ed25519.authKey);
  });
});
