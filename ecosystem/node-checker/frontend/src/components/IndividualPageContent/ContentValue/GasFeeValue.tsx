import React from "react";
import {grey} from "../../../themes/colors/aptosColorPalette";
import {APTCurrencyValue} from "./CurrencyValue";
import GasValue from "./GasValue";

type GasFeeValueProps = {
  gasUsed: string;
  gasUnitPrice: string;
  showGasUsed?: boolean;
};

export default function GasFeeValue({
  gasUsed,
  gasUnitPrice,
  showGasUsed,
}: GasFeeValueProps) {
  return (
    <>
      <APTCurrencyValue
        amount={(BigInt(gasUnitPrice) * BigInt(gasUsed)).toString()}
      />
      {showGasUsed === true && (
        <span style={{color: grey[450]}}>
          {" ("}
          <GasValue gas={gasUsed} />
          {")"}
        </span>
      )}
    </>
  );
}
