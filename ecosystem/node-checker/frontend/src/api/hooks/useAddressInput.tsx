import React, {useState, useEffect} from "react";
import AddressTextField from "../../components/AddressTextField";
import {isValidAccountAddress} from "../../pages/utils";

const useAddressInput = () => {
  const [addr, setAddr] = useState<string>("");
  const [addrIsValid, setAddrIsValid] = useState<boolean>(true);

  useEffect(() => {
    setAddrIsValid(true);
  }, [addr]);

  const onAddrChange = (event: React.ChangeEvent<HTMLInputElement>) => {
    setAddr(event.target.value);
  };

  function clearAddr() {
    setAddr("");
  }

  function renderAddressTextField(label: string): JSX.Element {
    return (
      <AddressTextField
        label={label}
        addr={addr}
        addrIsValid={addrIsValid}
        onAddrChange={onAddrChange}
      />
    );
  }

  function validateAddressInput(): boolean {
    const isValid = isValidAccountAddress(addr);
    setAddrIsValid(isValid);
    return isValid;
  }

  return {addr, clearAddr, renderAddressTextField, validateAddressInput};
};

export default useAddressInput;
