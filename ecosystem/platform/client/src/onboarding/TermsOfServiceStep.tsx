import * as React from "react";
import {TosAcceptance} from "./types";

type Props = {onComplete: (tosAcceptance: TosAcceptance) => void};

export class TermsOfServiceStep extends React.Component<Props> {
  render() {
    const {onComplete} = this.props;

    return (
      <div>
        <h3>Terms of Service</h3>
        <p>
          <textarea readOnly value="Lorem ipsum dolor sit amet" />
        </p>
        <p>
          <button onClick={() => onComplete({date: Date.now(), ip: "0.0.0.0"})}>
            I Accept
          </button>
        </p>
      </div>
    );
  }
}
