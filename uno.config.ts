import { defineConfig, presetUno } from 'unocss'

export default defineConfig({
  presets: [presetUno()],
  theme: {
    colors: {
      primary: '#7c3aed',
      surface: '#1e1e2e',
      text: '#cdd6f4',
    },
  },
})
