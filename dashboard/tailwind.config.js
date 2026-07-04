/** @type {import('tailwindcss').Config} */
export default {
  content: ['./index.html', './src/**/*.{vue,ts}'],
  theme: {
    extend: {
      colors: {
        vessel: {
          bg: '#0f0f0f',
          card: '#1a1a1a',
          border: '#2a2a2a',
          muted: '#a3a3a3',
        },
      },
    },
  },
  plugins: [],
}
