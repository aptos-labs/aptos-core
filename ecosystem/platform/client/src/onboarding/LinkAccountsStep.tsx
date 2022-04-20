import * as React from "react";
import {GitHubLoginButton} from "auth";
import {SocialAccount} from "./types";

type Props = {onComplete: (accounts: SocialAccount[]) => void};

export class LinkAccountsStep extends React.Component<Props> {
  render() {
    const {onComplete} = this.props;
    const fakeAccount: SocialAccount = {
      service: "github",
      username: "example",
    };

    return (
      <div>
        <h3>Link your account</h3>
        <p>
          <GitHubLoginButton onClick={() => onComplete([fakeAccount])} />
        </p>
      </div>
    );
  }
}
