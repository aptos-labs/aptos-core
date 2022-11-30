import React from "react";
import Box from "@mui/material/Box";
import {Card, CardContent} from "@mui/material";
import Typography from "@mui/material/Typography";
import Grid from "@mui/material/Grid";

const bull = (
  <Box
    component="span"
    sx={{display: "inline-block", mx: "2px", transform: "scale(0.8)"}}
  >
    •
  </Box>
);

export default function NotFoundPage() {
  return (
    <Grid
      container
      spacing={0}
      direction="column"
      style={{alignItems: "center"}}
      sx={{mt: 3}}
    >
      <Grid item xs={2}>
        <Card>
          <CardContent>
            <Typography sx={{fontSize: 14}} color="text.secondary" gutterBottom>
              Word of the Day
            </Typography>
            <Typography variant="h5" component="div">
              404{bull}Pa{bull}ge{bull}Not{bull}Fou{bull}nd
            </Typography>
            <Typography variant="body2">
              Maybe the page you are looking for has been removed, or you typed
              in the wrong URL.
              <br />
              You don't have to go home, but you can't stay here!
            </Typography>
          </CardContent>
        </Card>
      </Grid>
    </Grid>
  );
}
