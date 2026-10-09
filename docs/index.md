---
layout: page
---

<script setup>
import { onMounted } from 'vue'
import { withBase } from 'vitepress'

onMounted(() => {
  const userLang = (navigator.language || navigator.userLanguage || '').toLowerCase()
  if (userLang.startsWith('en')) {
    window.location.replace(withBase('/en/'))
  } else {
    window.location.replace(withBase('/zh/'))
  }
})
</script>

<div style="text-align: center; padding: 96px 24px; max-width: 680px; margin: 0 auto;">
  <h1 style="font-size: 2.25rem; font-weight: 700; color: #F8FAFC; margin-bottom: 16px;">
    K0maru Agent Memory
  </h1>
  <p style="font-size: 1.125rem; color: #94A3B8; margin-bottom: 36px; line-height: 1.6;">
    Redirecting to technical documentation...<br />
    正在跳转至技术文档...
  </p>
  <div style="display: flex; gap: 16px; justify-content: center; flex-wrap: wrap;">
    <a :href="withBase('/zh/')" style="display: inline-flex; align-items: center; padding: 10px 24px; background-color: #22C55E; color: #0F172A; font-weight: 600; border-radius: 6px; text-decoration: none;">
      🇨🇳 简体中文文档
    </a>
    <a :href="withBase('/en/')" style="display: inline-flex; align-items: center; padding: 10px 24px; background-color: #1B2336; border: 1px solid #334155; color: #F8FAFC; font-weight: 600; border-radius: 6px; text-decoration: none;">
      🌐 English Manual
    </a>
  </div>
</div>
