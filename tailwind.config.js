/** @type {import('tailwindcss').Config} */
module.exports = {
  content: [
    "./crates/manager-ui/src/**/*.rs",
  ],
  theme: {
    extend: {
      colors: {
        slate: {
          750: '#293548',
        },
      },
    },
  },
  plugins: [],
}
