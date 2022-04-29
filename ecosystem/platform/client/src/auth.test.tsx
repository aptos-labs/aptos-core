// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {render} from "@testing-library/react";
import {AuthContext, useAuth} from "auth";

describe("useAuth", () => {
  it("when userId is undefined", () => {
    const Mock = () => {
      const {isLoaded, isSignedIn, userId} = useAuth();

      expect(isLoaded).toBe(false);
      expect(isSignedIn).toBe(undefined);
      expect(userId).toBe(undefined);

      return null;
    };

    render(
      <AuthContext.Provider value={{userId: undefined, setUserId: jest.fn()}}>
        <Mock />
      </AuthContext.Provider>,
    );
  });

  it("when userId is null", () => {
    const Mock = () => {
      const {isLoaded, isSignedIn, userId} = useAuth();

      expect(isLoaded).toBe(true);
      expect(isSignedIn).toBe(false);
      expect(userId).toBe(null);

      return null;
    };

    render(
      <AuthContext.Provider value={{userId: null, setUserId: jest.fn()}}>
        <Mock />
      </AuthContext.Provider>,
    );
  });

  it("when userId is a string", () => {
    const Mock = () => {
      const {isLoaded, isSignedIn, userId} = useAuth();

      expect(isLoaded).toBe(true);
      expect(isSignedIn).toBe(true);
      expect(userId).toBe("example");

      return null;
    };

    render(
      <AuthContext.Provider value={{userId: "example", setUserId: jest.fn()}}>
        <Mock />
      </AuthContext.Provider>,
    );
  });
});

describe("AuthProvider", () => {
  // TODO: When server-side auth routes exist, test the integration here.
});
