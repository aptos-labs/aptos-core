import * as React from "react";
import {AptosAddress} from "./types";

type Props = {onComplete: (aptosAddress: AptosAddress) => void};

export class AptosAddressStep extends React.Component<Props> {
  render() {
    const {onComplete} = this.props;
    const fakeAddress: AptosAddress = "0x1";

    return (
      <div>
        <h3>Link your account</h3>
        <p>
          <input placeholder="0x1"></input>
        </p>
        <p>
          <button onClick={() => onComplete(fakeAddress)}>Submit</button>
        </p>
      </div>
    );
  }
}
