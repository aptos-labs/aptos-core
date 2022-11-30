import {Grid, Typography} from "@mui/material";
import * as React from "react";
import {useGetInGtmMode} from "../api/hooks/useGetInDevMode";
import DividerHero from "./DividerHero";
import GoBack from "./GoBack";

export default function PageHeader() {
  const inGtm = useGetInGtmMode();
  return <>{inGtm ? <GoBack /> : null}</>;
}
