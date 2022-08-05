const defaultTheme = require("tailwindcss/defaultTheme");

module.exports = {
  content: [
    "./app/components/**/*.{rb,erb}",
    "./app/helpers/**/*.rb",
    "./app/form_builders/*.rb",
    "./app/javascript/**/*.js",
    "./app/views/**/*.{erb,haml,html,slim}",
  ],
  theme: {
    extend: {
      colors: {
        neutral: {
          50: "#fafafa",
          100: "#f4f4f5",
          200: "#e4e4e7",
          300: "#d4d4d8",
          400: "#a1a1aa",
          500: "#4f5352",
          600: "#363a39",
          700: "#272b2a",
          800: "#1b1f1e",
          900: "#121615",
        },
        teal: {
          50: "#eafff7",
          100: "#cbffeb",
          200: "#9cfedc",
          300: "#5df8cc",
          400: "#2ed8a7",
          500: "#00d0a1",
          600: "#00a380",
          700: "#008068",
          800: "#005c4b",
          900: "#002e26",
        },
      },
      fontFamily: {
        sans: ["apparat", ...defaultTheme.fontFamily.sans],
        mono: ["lft-etica-mono", ...defaultTheme.fontFamily.mono],
        display: ["apparat-semicond", ...defaultTheme.fontFamily.sans],
      },
      screens: {
        'max-sm': {'max': '639px'},
      }
    },
    container: {
      padding: {
        DEFAULT: "1rem",
        sm: "2rem",
      },
    },
  },
  plugins: [require("@tailwindcss/forms"), require("@tailwindcss/typography")],
};
