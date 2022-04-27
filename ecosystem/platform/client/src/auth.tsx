// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import * as React from "react";

type AuthContextValue = {
  userId: string | null | undefined;
  setUserId: React.Dispatch<React.SetStateAction<string | null | undefined>>;
};

export const AuthContext = React.createContext<AuthContextValue | undefined>(
  undefined,
);

export function useAuthContext(): AuthContextValue {
  const context = React.useContext(AuthContext);

  if (!context) {
    throw new Error(
      "useAuthContext must be used within an AuthContext provider.",
    );
  }

  return context;
}

type Props = {children: React.ReactNode};

export function AuthProvider(props: Props) {
  const [userId, setUserId] =
    React.useState<AuthContextValue["userId"]>(undefined);
  const value = React.useMemo(() => {
    return {
      userId,
      setUserId,
    };
  }, [userId]);
  return <AuthContext.Provider value={value} {...props} />;
}

type Auth =
  | {
      isLoaded: false;
      isSignedIn: undefined;
      userId: undefined;
    }
  | {
      isLoaded: true;
      isSignedIn: false;
      userId: null;
    }
  | {
      isLoaded: true;
      isSignedIn: true;
      userId: string;
    };

export function useAuth(): Auth {
  const {userId} = useAuthContext();

  if (userId === undefined) {
    return {
      isLoaded: false,
      isSignedIn: undefined,
      userId: undefined,
    };
  }

  if (userId === null) {
    return {
      isLoaded: true,
      isSignedIn: false,
      userId: null,
    };
  }

  if (typeof userId === "string" && userId.length > 0) {
    return {
      isLoaded: true,
      isSignedIn: true,
      userId,
    };
  }

  throw new Error("Unable to determine authentication state.");
}
