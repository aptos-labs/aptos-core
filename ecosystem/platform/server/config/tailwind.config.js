const defaultTheme = require('tailwindcss/defaultTheme')

module.exports = {
  content: [
    './app/helpers/**/*.rb',
    './app/javascript/**/*.js',
    './app/views/**/*.{erb,haml,html,slim}'
  ],
  theme: {
    extend: {
      colors: {
        'neutral': {
          ...defaultTheme.colors.neutral,
          '700': '#393939',
          '800': '#212121',
          '900': '#171717',
        },
      },
      fontFamily: {
        'sans': ['apparat', ...defaultTheme.fontFamily.sans],
        'mono': ['lft-etica-mono', ...defaultTheme.fontFamily.mono],
        'display': ['apparat-semicond', ...defaultTheme.fontFamily.sans],
      },
    },
  },
  plugins: [
    require('@tailwindcss/forms'),
    require('@tailwindcss/aspect-ratio'),
    require('@tailwindcss/typography'),
  ]
}
