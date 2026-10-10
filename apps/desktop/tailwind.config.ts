import type { Config } from 'tailwindcss'

export default {
  content: ['./index.html', './src/**/*.{ts,tsx}'],
  darkMode: ['class'],
  theme: {
    extend: {
      colors: {
        brand: {
          bg: '#0B1220',
          'bg-secondary': '#101C2E',
          surface: '#14243A',
          'blue-deep': '#1E3A5F',
          cyan: '#00E5FF',
          'blue-accent': '#38B2F6',
          text: '#E5E7EB',
          'text-secondary': '#94A3B8',
          success: '#10B981',
          warning: '#F59E0B',
          danger: '#EF4444',
        },
      },
      fontFamily: {
        display: ['"Exo 2"', 'ui-sans-serif', 'system-ui', 'sans-serif'],
        sans: ['Inter', 'ui-sans-serif', 'system-ui', 'sans-serif'],
      },
    },
  },
  plugins: [],
} satisfies Config
