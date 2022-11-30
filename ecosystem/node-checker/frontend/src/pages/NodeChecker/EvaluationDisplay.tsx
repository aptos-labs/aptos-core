import React from "react";
import {
  Grid,
  Box,
  Card,
  Divider,
  Typography,
  useTheme,
  Button,
} from "@mui/material";
import {EvaluationSummary} from "aptos-node-checker-client";

interface EvaluationDisplayProps {
  evaluationSummary: EvaluationSummary;
}

type CardBoxProps = {
  title: string;
  content: string;
  links: string[];
};

export default function EvaluationDisplay({
  evaluationSummary,
}: EvaluationDisplayProps) {
  const theme = useTheme();

  const CardBox = ({title, content, links}: CardBoxProps): JSX.Element => {
    let linkButton = null;
    // We only show a button for one link right now.
    if (links.length > 0) {
      linkButton = (
        <a href={links[0]} target="_blank" rel="noreferrer noopener">
          <Button variant="primary">More information</Button>
        </a>
      );
    }

    return (
      <Card>
        <Box
          minHeight={320}
          sx={{
            display: "flex",
            flexDirection: "column",
            margin: 3,
          }}
        >
          <Box>
            <Typography variant="h6" sx={{color: "#1de9b6"}}>
              {title}
            </Typography>
            <Divider
              variant={theme.palette.mode === "dark" ? "bumpDark" : "bump"}
              sx={{mt: 2}}
            />
          </Box>
          <Typography
            variant="body1"
            marginBottom={2}
            sx={{
              wordWrap: "break-word",
            }}
          >
            {content}
          </Typography>
          {linkButton}
        </Box>
      </Card>
    );
  };

  // Get grid components for each element in the evaluation summary results.
  const results = evaluationSummary.evaluation_results.map((result, index) => {
    return (
      <Grid item xs={12} md={6} lg={4} key={index}>
        <CardBox
          title={`${result.score}: ${result.headline}`}
          content={result.explanation}
          links={result.links}
        />
      </Grid>
    );
  });

  return (
    <Box marginX={6}>
      <Grid md={12}>
        <Typography
          variant="h4"
          marginTop={5}
          marginBottom={6}
          textAlign="center"
        >
          {evaluationSummary.summary_explanation}
        </Typography>
        <Grid container spacing={4} justifyContent="center">
          {results}
        </Grid>
      </Grid>
    </Box>
  );
}
