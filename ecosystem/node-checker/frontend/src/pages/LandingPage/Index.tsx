import React from "react";
import Typography from "@mui/material/Typography";
import {Box, Button, Divider, Grid} from "@mui/material";
import DividerHero from "../../components/DividerHero";

export default function LandingPage() {
  return (
    <Box>
      <Typography variant="h3" component="h3" marginBottom={4}>
        Aptos Node Tools
      </Typography>
      <h2>BETA</h2>
      <DividerHero/>
      <Grid item xs={12} md={6} lg={4} key={1}>
        <a href="/node_checker" style={{ textDecoration: 'none' }}>
          <Button variant="primary">Node Checker</Button>
        </a>
      </Grid>
    </Box>
  );
}
