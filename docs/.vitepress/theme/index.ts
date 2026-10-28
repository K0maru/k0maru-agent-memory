import DefaultTheme from 'vitepress/theme'
import type { Theme } from 'vitepress'
import { h } from 'vue'
import AsciiHeroEngine from './AsciiHeroEngine.vue'
import './custom.css'

export default {
  extends: DefaultTheme,
  Layout: () => {
    return h(DefaultTheme.Layout, null, {
      'home-hero-before': () => h(AsciiHeroEngine, { mode: 'all' }),
    })
  },
  enhanceApp({ app }) {
    app.component('AsciiHeroEngine', AsciiHeroEngine)
  },
} satisfies Theme
