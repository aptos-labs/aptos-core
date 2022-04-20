import * as React from "react";
import {Identity} from "./types";

type Props = {identity: Identity};

export class OnboardingCompleteStep extends React.Component<Props> {
  render() {
    const {identity} = this.props;

    return (
      <div>
        <h3>Success</h3>
        <p>Lorem ipsum dolor sit amet</p>
        <pre>{JSON.stringify(identity)}</pre>
      </div>
    );
  }
}
