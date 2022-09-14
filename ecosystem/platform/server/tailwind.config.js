const defaultTheme = require("tailwindcss/defaultTheme");

module.exports = {
  content: [
    "./app/components/**/*.{rb,erb}",
    "./app/helpers/**/*.rb",
    "./app/controllers/**/*.rb",
    "./app/form_builders/*.rb",
    "./app/javascript/**/*.js",
    "./app/views/**/*.{erb,haml,html,slim}",
  ],
  theme: {
    extend: {
      colors: {
        neutral: {
          ...defaultTheme.colors.neutral,
          100: "#f5f5f5",
          700: "#414141",
          800: "#262626",
          900: "#171717",
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
      fontWeight: {
        normal: 300,
      },
      screens: {
        "max-sm": { max: "767px" },
      },
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
