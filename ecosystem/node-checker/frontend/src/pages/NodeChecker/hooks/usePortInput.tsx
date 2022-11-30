import React, {useState, useEffect} from "react";
import PortTextField from "../../../components/PortTextField";

function isValidPort(port: string): boolean {
  let portNumber;
  try {
    portNumber = parseInt(port);
  } catch (_) {
    return false;
  }

  return portNumber >= 1 && portNumber <= 65535;
}

const usePortInput = (initialValue: string) => {
  const [port, setPort] = useState<string>(initialValue);
  const [portIsValid, setPortIsValid] = useState<boolean>(true);

  useEffect(() => {
    setPortIsValid(true);
  }, [port]);

  const onPortChange = (event: React.ChangeEvent<HTMLInputElement>) => {
    setPort(event.target.value.replace(/[^0-9]/g, ""));
  };

  function clearPort() {
    setPort("");
  }

  function renderPortTextField(label: string): JSX.Element {
    return (
      <PortTextField
        label={label}
        port={port}
        portIsValid={portIsValid}
        errorMessage={`Port is invalid`}
        onPortChange={onPortChange}
      />
    );
  }

  function validatePortInput(): boolean {
    const isValid = isValidPort(port);
    setPortIsValid(isValid);
    return isValid;
  }

  return {port, clearPort, renderPortTextField, validatePortInput};
};

export default usePortInput;
