import {PaletteMode} from "@mui/material";
import {ThemeOptions} from "@mui/material/styles";
import "@mui/material/styles/createPalette";
import {alpha} from "@mui/material";
import {grey, primary} from "./colors/aptosColorPalette";

// Button variants
declare module "@mui/material/Button" {
  interface ButtonPropsVariantOverrides {
    nav: true;
    primary: true;
  }
}

// Divider variant - dotted
declare module "@mui/material/Divider" {
  interface DividerPropsVariantOverrides {
    dotted: true;
  }
}

declare module "@mui/material/styles" {
  interface TypographyVariants {
    stats: React.CSSProperties;
  }
  // allow configuration using `createTheme`
  interface TypographyVariantsOptions {
    stats?: React.CSSProperties;
  }
}

declare module "@mui/material/styles/createPalette" {
  interface Palette {
    lineShade: {
      main: string;
    };
    neutralShade: {
      main: string;
      lighter: string;
      darker: string;
    };
  }
}

// Divider variant - big stats
declare module "@mui/material/Typography" {
  interface TypographyPropsVariantOverrides {
    stats: true;
  }
}

// Divider variant - bump
declare module "@mui/material/Divider" {
  interface DividerPropsVariantOverrides {
    bump: true;
    bumpDark: true;
    bumpRight: true;
    bumpRightDark: true;
  }
}

const primaryColor = primary["400"];
const primaryColorToned = primary["600"];

const getDesignTokens = (mode: PaletteMode): ThemeOptions => ({
  shape: {
    borderRadius: 12,
  },
  //

  typography: {
    fontFamily: `lft-etica-mono,ui-monospace,SFMono-Regular,SF Mono,Menlo,Consolas,Liberation Mono,monospace`,
    fontWeightLight: 200,
    fontWeightRegular: 400,
    fontWeightBold: 500,
    h1: {
      fontFamily: `apparat-semicond,Geneva,Tahoma,Verdana,sans-serif`,
      fontWeight: "600",
    },
    h2: {
      fontFamily: `apparat-semicond,Geneva,Tahoma,Verdana,sans-serif`,
      fontWeight: "600",
    },
    h3: {
      fontFamily: `apparat-semicond,Geneva,Tahoma,Verdana,sans-serif`,
      fontWeight: "600",
    },
    h4: {
      fontFamily: `apparat-semicond,Geneva,Tahoma,Verdana,sans-serif`,
      fontWeight: "600",
    },
    h5: {
      fontFamily: `apparat-semicond,Geneva,Tahoma,Verdana,sans-serif`,
      fontWeight: "600",
    },
    h6: {
      fontFamily: `apparat-semicond,Geneva,Tahoma,Verdana,sans-serif`,
      fontWeight: "600",
    },
    stats: {
      fontFamily: `lft-etica-mono,ui-monospace,SFMono-Regular,SF Mono,Menlo,Consolas,Liberation Mono,monospace`,
      fontWeight: "400",
    },
    subtitle1: {
      fontWeight: 400,
      textTransform: "uppercase",
      lineHeight: "1.25",
    },
    subtitle2: {
      fontWeight: 400,
      fontSize: "1rem",
      textTransform: "capitalize",
      lineHeight: "1.25",
    },
  },

  palette: {
    mode,
    ...(mode === "light"
      ? {
          // light mode palette values
          primary: {
            main: primaryColorToned,
          },

          secondary: {
            main: grey[700],
          },

          success: {
            main: primaryColorToned,
          },

          background: {
            default: "#ffffff",
            paper: grey[100],
          },

          text: {
            primary: grey[900],
            secondary: grey[500],
          },

          lineShade: {
            main: grey[200],
          },

          neutralShade: {
            main: grey[50],
            darker: grey[100],
          },
        }
      : {
          // dark mode palette values
          primary: {
            main: primaryColor,
          },

          secondary: {
            main: grey[300],
          },

          success: {
            main: primaryColor,
          },

          background: {
            default: grey[900],
            paper: grey[800],
          },

          text: {
            primary: grey[50],
            secondary: grey[200],
          },

          lineShade: {
            main: grey[700],
          },

          neutralShade: {
            main: grey[800],
            lighter: grey[700],
          },
        }),
  },

  components: {
    // Typography overrides
    MuiTypography: {
      styleOverrides: {
        subtitle2: {
          display: "block",
        },
      },
    },
    // Autocomplete overrides
    MuiAutocomplete: {
      styleOverrides: {
        root: ({theme}) => ({
          listbox: {
            padding: "0",
          },
        }),
      },
    },

    // Link overrides
    MuiLink: {
      styleOverrides: {
        root: {
          fontWeight: "400",
        },
      },
    },

    // Paper overrides
    MuiPaper: {
      styleOverrides: {
        root: ({theme}) => ({
          backgroundImage: "none",
          borderRadius: theme.shape.borderRadius,
          transition: "none !important",
          boxShadow: "none",
        }),
      },
    },
    MuiPopover: {
      styleOverrides: {
        paper: {
          boxShadow: "0 25px 50px -12px rgba(0,0,0,0.25)",
        },
      },
    },

    MuiInput: {
      styleOverrides: {
        root: ({theme}) => ({
          borderRadius: 2,
        }),
      },
    },

    MuiFilledInput: {
      styleOverrides: {
        root: ({theme}) => ({
          borderRadius: `${theme.shape.borderRadius}px`,
          backgroundColor: `${
            theme.palette.mode === "dark" ? grey[700] : grey[100]
          }`,
          transition: "none",
          "&.Mui-focused": {
            filter: `${
              theme.palette.mode === "dark"
                ? "brightness(1.1)"
                : "brightness(0.98)"
            }`,
            boxShadow: `0 0 0 3px ${alpha(primaryColor, 0.5)}`,
          },
          "&:hover": {
            backgroundColor: `${
              theme.palette.mode === "dark" ? grey[800] : grey[50]
            }`,
            filter: `${
              theme.palette.mode === "dark"
                ? "brightness(1.1)"
                : "brightness(0.99)"
            }`,
          },
        }),
      },
    },

    MuiOutlinedInput: {
      styleOverrides: {
        root: ({theme}) => ({
          "&.Mui-focused": {
            boxShadow: `0 0 0 2px ${alpha(
              theme.palette.mode === "dark" ? primaryColor : primaryColorToned,
              0.35,
            )}`,
          },
          ".MuiOutlinedInput-notchedOutline": {
            borderColor: theme.palette.lineShade.main,
          },
          "&:hover .MuiOutlinedInput-notchedOutline, &.Mui-focused .MuiOutlinedInput-notchedOutline":
            {
              borderColor: `${alpha(
                theme.palette.mode === "dark"
                  ? primaryColor
                  : primaryColorToned,
                0.35,
              )}`,
            },
        }),
      },
    },

    // Select overrides
    MuiSelect: {
      styleOverrides: {
        select: {
          borderRadius: "8px",
          textTransform: "capitalize",
        },
        outlined: {
          backgroundColor: "transparent",
        },
      },
    },
    MuiList: {
      styleOverrides: {
        root: {
          padding: 5,
          mt: 5,
        },
      },
    },
    MuiMenuItem: {
      styleOverrides: {
        root: ({theme}) => ({
          borderRadius: theme.shape.borderRadius,
          textTransform: "capitalize",
        }),
      },
    },

    // Divider overrides
    MuiDivider: {
      variants: [
        {
          props: {variant: "dotted"},
          style: {
            borderStyle: "dotted",
            borderWidth: "0 0 2px",
          },
        },
        {
          props: {variant: "bump"},
          style: {
            transform: "translateY(-20px)",
            border: `0`,
            height: "20px",
            background: "transparent",
            position: "relative",
            "&::before, &::after": {
              content: '""',
              position: "absolute",
              bottom: "0",
              willChange: "transform",
            },
            "&::before": {
              display: "block",
              width: "100%",
              height: "1px",
              background: `${primaryColorToned}`,
              maskClip: "content-box",
              maskImage: `-webkit-linear-gradient(black, black),
            url('data:image/svg+xml;utf8,<svg xmlns="http://www.w3.org/2000/svg" width="16" height="6" viewBox="0 0 14 6"><rect fill="black" x="0" y="0" width="13" height="6"></rect></svg>')`,
              maskPosition: `0 0, 20%`,
              maskRepeat: "no-repeat",
              maskComposite: "exclude",
              WebkitMaskComposite: "xor",
            },
            "&::after": {
              width: "14px",
              height: "6px",
              backgroundSize: "100%",
              backgroundRepeat: "none",
              backgroundImage: `url('data:image/svg+xml;utf8,<svg xmlns="http://www.w3.org/2000/svg" width="14" height="6" viewBox="0 0 14 6"><path d="M0,5.5a2.09,2.09,0,0,0,1.51-.64L4.66,1.53a3.36,3.36,0,0,1,4.69-.15,2.28,2.28,0,0,1,.22.22l2.88,3.21A2.08,2.08,0,0,0,14,5.5" fill="none" stroke="%23${primaryColorToned.substring(
                1,
              )}" stroke-miterlimit="10"/></svg>')`,
              transform: `translateX(calc(-1 * 20%))`,
              left: "20%",
            },
          },
        },
        {
          props: {variant: "bumpDark"},
          style: {
            transform: "translateY(-20px)",
            border: `0`,
            height: "20px",
            background: "transparent",
            position: "relative",
            "&::before, &::after": {
              content: '""',
              position: "absolute",
              bottom: "0",
              willChange: "transform",
            },
            "&::before": {
              display: "block",
              width: "100%",
              height: "1px",
              background: `${primaryColor}`,
              maskClip: "content-box",
              maskImage: `-webkit-linear-gradient(black, black),
            url('data:image/svg+xml;utf8,<svg xmlns="http://www.w3.org/2000/svg" width="16" height="6" viewBox="0 0 14 6"><rect fill="white" x="0" y="0" width="13" height="6"></rect></svg>')`,
              maskPosition: `0 0, 20%`,
              maskRepeat: "no-repeat",
              maskComposite: "exclude",
              WebkitMaskComposite: "xor",
            },
            "&::after": {
              width: "14px",
              height: "6px",
              backgroundSize: "100%",
              backgroundRepeat: "none",
              backgroundImage: `url('data:image/svg+xml;utf8,<svg xmlns="http://www.w3.org/2000/svg" width="14" height="6" viewBox="0 0 14 6"><path d="M0,5.5a2.09,2.09,0,0,0,1.51-.64L4.66,1.53a3.36,3.36,0,0,1,4.69-.15,2.28,2.28,0,0,1,.22.22l2.88,3.21A2.08,2.08,0,0,0,14,5.5" fill="none" stroke="%23${primaryColor.substring(
                1,
              )}" stroke-miterlimit="10"/></svg>')`,
              transform: `translateX(calc(-1 * 20%))`,
              left: "20%",
            },
          },
        },
        {
          props: {variant: "bumpRight"},
          style: {
            marginTop: "-20px",
            border: `0`,
            height: "20px",
            background: "transparent",
            marginBottom: "4rem",
            position: "relative",
            "&::before, &::after": {
              content: '""',
              position: "absolute",
              bottom: "0",
              willChange: "transform",
            },
            "&::before": {
              display: "block",
              width: "100%",
              height: "1px",
              background: "black",
              maskClip: "content-box",
              maskImage: `-webkit-linear-gradient(black, black),
            url('data:image/svg+xml;utf8,<svg xmlns="http://www.w3.org/2000/svg" width="16" height="6" viewBox="0 0 14 6"><rect fill="black" x="0" y="0" width="13" height="6"></rect></svg>')`,
              maskPosition: `0 0, 78%`,
              maskRepeat: "no-repeat",
              maskComposite: "exclude",
              WebkitMaskComposite: "xor",
            },
            "&::after": {
              width: "14px",
              height: "6px",
              backgroundSize: "100%",
              backgroundRepeat: "none",
              backgroundImage: `url('data:image/svg+xml;utf8,<svg xmlns="http://www.w3.org/2000/svg" width="14" height="6" viewBox="0 0 14 6"><path d="M0,5.5a2.09,2.09,0,0,0,1.51-.64L4.66,1.53a3.36,3.36,0,0,1,4.69-.15,2.28,2.28,0,0,1,.22.22l2.88,3.21A2.08,2.08,0,0,0,14,5.5" fill="none" stroke="black" stroke-miterlimit="10"/></svg>')`,
              transform: `translateX(calc(-1 * 78%))`,
              left: "78%",
            },
          },
        },
        {
          props: {variant: "bumpRightDark"},
          style: {
            transform: "translateY(-20px)",
            border: `0`,
            height: "20px",
            background: "transparent",
            position: "relative",
            "&::before, &::after": {
              content: '""',
              position: "absolute",
              bottom: "0",
              willChange: "transform",
            },
            "&::before": {
              display: "block",
              width: "100%",
              height: "1px",
              background: `${primaryColor}`,
              maskClip: "content-box",
              maskImage: `-webkit-linear-gradient(black, black),
            url('data:image/svg+xml;utf8,<svg xmlns="http://www.w3.org/2000/svg" width="16" height="6" viewBox="0 0 14 6"><rect fill="white" x="0" y="0" width="13" height="6"></rect></svg>')`,
              maskPosition: `0 0, 78%`,
              maskRepeat: "no-repeat",
              maskComposite: "exclude",
              WebkitMaskComposite: "xor",
            },
            "&::after": {
              width: "14px",
              height: "6px",
              backgroundSize: "100%",
              backgroundRepeat: "none",
              backgroundImage: `url('data:image/svg+xml;utf8,<svg xmlns="http://www.w3.org/2000/svg" width="14" height="6" viewBox="0 0 14 6"><path d="M0,5.5a2.09,2.09,0,0,0,1.51-.64L4.66,1.53a3.36,3.36,0,0,1,4.69-.15,2.28,2.28,0,0,1,.22.22l2.88,3.21A2.08,2.08,0,0,0,14,5.5" fill="none" stroke="%23${primaryColor}" stroke-miterlimit="10"/></svg>')`,
              transform: `translateX(calc(-1 * 78%))`,
              left: "78%",
            },
          },
        },
      ],
    },

    // Table overrides
    MuiTable: {
      styleOverrides: {
        root: {
          borderCollapse: "separate",
          borderSpacing: "0px 0.5rem",
        },
      },
    },

    // Table Head overrides
    MuiTableHead: {
      styleOverrides: {
        root: {
          borderBottomWidth: "0",
          background: "transparent",
          borderSpacing: "0px",
        },
      },
    },

    // Table Body overrides
    MuiTableBody: {
      styleOverrides: {
        root: {
          position: "relative",
          "&::before": {
            content: '""',
            display: "block",
            height: 10,
          },
        },
      },
    },

    MuiTableRow: {
      styleOverrides: {
        head: {
          background: "transparent",
        },
      },
    },

    // Table Cell overrides
    MuiTableCell: {
      styleOverrides: {
        head: {
          border: "0",
          background: "transparent",
          paddingBottom: "0",
        },
        root: ({theme}) => ({
          padding: "0.75rem 1.5rem 0.75rem 1.5rem",
          whiteSpace: "nowrap",
          borderStyle: "solid",
          borderWidth: "0 0 0 0",
          borderColor: grey[700],
          "&:first-of-type": {
            borderRadius: `${theme.shape.borderRadius}px 0 0 ${theme.shape.borderRadius}px`,
          },
          "&:last-of-type": {
            borderRadius: `0 ${theme.shape.borderRadius}px ${theme.shape.borderRadius}px 0`,
          },
        }),
      },
    },

    // Button overrides
    MuiButtonBase: {
      defaultProps: {
        disableRipple: true,
      },
    },
    MuiButton: {
      defaultProps: {
        disableElevation: true,
        disableFocusRipple: true,
        disableRipple: true,
      },
      styleOverrides: {
        root: {
          transition: "none !important",
          fontWeight: "400",
          "&:hover": {
            filter: "brightness(0.98)",
          },
          "&.Mui-disabled": {
            opacity: 0.5,
            color: "black",
          },
        },
      },
      variants: [
        {
          props: {variant: "primary"},
          style: ({theme}) => ({
            backgroundColor:
              theme.palette.mode === "dark" ? primaryColor : primary["500"],
            color: "black",
            fontSize: "1.1rem",
            padding: "12px 34px",
            minWidth: "8rem",
            "&:hover": {
              backgroundColor: alpha(primaryColor, 1),
            },
          }),
        },
        {
          props: {variant: "nav"},
          style: {
            textTransform: "capitalize",
            color: grey[300],
            fontSize: "1rem",
            fontWeight: "normal",
            "&:hover": {
              background: "transparent",
              opacity: "0.8",
            },
            "&.active": {},
          },
        },
      ],
    },
  },
});

export default getDesignTokens;
