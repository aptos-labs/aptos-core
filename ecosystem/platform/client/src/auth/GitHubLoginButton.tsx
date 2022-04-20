import * as React from "react";

type Props = {
  [prop: string]: any;
};

export class GitHubLoginButton extends React.Component<Props> {
  render() {
    return <button {...this.props}>Log in with GitHub</button>;
  }
}
