const defaultTheme = require('tailwindcss/defaultTheme')

module.exports = {
  content: [
    './app/components/**/*.{rb,erb}',
    './app/helpers/**/*.rb',
    './app/form_builders/*.rb',
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
        'teal': {
          ...defaultTheme.colors.teal,
          '300': '#1de9b6',
          '400': '#1bd7a4',
          '500': '#187e65',
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
