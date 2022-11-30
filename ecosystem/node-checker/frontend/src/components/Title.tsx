import React from "react";
import Typography from "@mui/material/Typography";
import {SxProps} from "@mui/system";
import {Theme} from "@mui/material/styles";

interface TitleProps {
  children?: React.ReactNode;
  sx?: SxProps<Theme>;
}

export default function Title(props: TitleProps) {
  return (
    <Typography component="h2" variant="h4" gutterBottom sx={props.sx}>
      {props.children}
    </Typography>
  );
}
