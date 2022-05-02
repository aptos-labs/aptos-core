// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {act, fireEvent, render, screen} from "@testing-library/react";
import * as React from "react";

import {SocialLoginButton} from "./SocialLoginButton";

const TEST_UUID = "11c16097-505b-4622-8b8d-9480bb52a024";

jest.useFakeTimers();

jest.mock("utils", () => {
  return {
    randomUUID: () => {
      return TEST_UUID;
    },
  };
});

describe("github", () => {
  it('renders a "Sign in with GitHub" button', () => {
    render(<SocialLoginButton service="github" onSuccess={jest.fn()} />);
    const button = screen.getByRole("button");
    expect(button).toHaveTextContent("Sign in with GitHub");
  });

  it("opens the github oauth page in a new window when clicked", () => {
    global.open = jest.fn();
    render(<SocialLoginButton service="github" onSuccess={jest.fn()} />);

    const button = screen.getByRole("button");
    fireEvent.click(button);

    expect(global.open).toHaveBeenCalledWith(
      `https://github.com/login/oauth/authorize?scope=user:email&client_id=test_github_client_id&state=${TEST_UUID}`,
      "aptos-oauth-popup",
    );
  });

  it("calls onSuccess when the oauth flow is completed", async () => {
    const onSuccess = jest.fn();
    const oauthPopup = jest.mocked({
      name: "aptos-oauth-popup",
    }) as jest.Mocked<Window>;
    global.open = () => {
      // Wait until after the component sets up the event listener to fire the event.
      setTimeout(() => {
        fireEvent(
          window,
          new MessageEvent("message", {
            origin: window.location.origin,
            source: oauthPopup,
            data: `?token=example_token&state=${TEST_UUID}`,
          }),
        );
      }, 1);

      return oauthPopup;
    };
    render(<SocialLoginButton service="github" onSuccess={onSuccess} />);

    const button = screen.getByRole("button");
    fireEvent.click(button);
    act(() => {
      jest.runAllTimers();
    });

    expect(onSuccess).toHaveBeenCalled();
  });
});
