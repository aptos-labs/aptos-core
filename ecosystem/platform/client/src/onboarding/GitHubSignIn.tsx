import * as React from "react";
import {useAuthContext} from "auth";
import {SocialLoginButton} from "ui";

export function GitHubSignIn() {
  const {setUserId} = useAuthContext();
  return (
    <div>
      <SocialLoginButton
        onClick={() => setUserId("example")}
        service="github"
        id="github"
      />
    </div>
  );
}
