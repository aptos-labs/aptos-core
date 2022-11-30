import {ThemeProvider} from "@mui/system";
import React, {createContext, useContext} from "react";
import useProvideColorMode, {ColorModeContext} from "./ProvideColorMode.State";

interface ProvideColorModeProps {
  children: React.ReactNode;
}

const colorModeContext = createContext<ColorModeContext | null>(null);

export const ProvideColorMode: React.FC<ProvideColorModeProps> = ({
  children,
}: ProvideColorModeProps) => {
  const {toggleColorMode, theme} = useProvideColorMode();

  return (
    <colorModeContext.Provider value={{toggleColorMode}}>
      <ThemeProvider theme={theme}>{children}</ThemeProvider>
    </colorModeContext.Provider>
  );
};

export const useColorMode = (): ColorModeContext => {
  const context = useContext(colorModeContext) as ColorModeContext;
  if (!context) {
    throw new Error("useColorMode must be used within a ColorModeContext");
  }
  return context;
};
