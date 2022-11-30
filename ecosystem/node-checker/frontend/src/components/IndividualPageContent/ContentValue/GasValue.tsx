import React from "react";
import NumberFormat from "react-number-format";

type GasValueProps = {
  gas: string;
};

export default function GasValue({gas}: GasValueProps) {
  return (
    <span>
      <NumberFormat value={gas} displayType="text" thousandSeparator /> Gas
      Units
    </span>
  );
}
