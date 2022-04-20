import * as React from "react";
import {Persona} from "./types";

type Props = {onComplete: (personas: Persona[]) => void};

export class SelectPersonaStep extends React.Component<Props> {
  render() {
    const {onComplete} = this.props;
    return (
      <div>
        <h3>Welcome to Aptos</h3>
        <p>Lorem ipsum dolor sit amet</p>
        <p>
          <button onClick={() => onComplete(["operator"])}>Get Started</button>
        </p>
      </div>
    );
  }
}
