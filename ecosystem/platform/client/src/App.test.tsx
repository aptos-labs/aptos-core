// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {render, screen} from "@testing-library/react";
import * as React from "react";
import {MemoryRouter} from "react-router";

import {App} from "./App";

jest.mock("onboarding", () => {
  return {
    OnboardingPage: () => <p>OnboardingPage</p>,
  };
});

jest.mock("ui", () => {
  return {
    SocialLoginButtonCallbackPage: () => <p>SocialLoginButtonCallbackPage</p>,
  };
});

it("renders the OnboardingPage at /onboarding", () => {
  render(
    <MemoryRouter initialEntries={["/onboarding"]}>
      <App />
    </MemoryRouter>,
  );

  expect(screen.getByText("OnboardingPage")).toBeInTheDocument();
});

it("renders the SocialLoginButtonCallbackPage at /oauth", () => {
  render(
    <MemoryRouter initialEntries={["/oauth"]}>
      <App />
    </MemoryRouter>,
  );

  expect(screen.getByText("SocialLoginButtonCallbackPage")).toBeInTheDocument();
});
