---
layout: page
---

<script setup>
import { onMounted } from 'vue'
import { withBase } from 'vitepress'

onMounted(() => {
  const userLang = (navigator.language || navigator.userLanguage || '').toLowerCase()
  if (userLang.startsWith('zh')) {
    window.location.replace(withBase('/zh/'))
  } else {
    window.location.replace(withBase('/en/'))
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
      🇨🇳 简体中文
    </a>
    <a :href="withBase('/en/')" style="display: inline-flex; align-items: center; padding: 10px 24px; background-color: #1B2336; border: 1px solid #334155; color: #F8FAFC; font-weight: 600; border-radius: 6px; text-decoration: none;">
      🌐 English
    </a>
  </div>
  <noscript>
    <p style="color: #64748B; margin-top: 24px; font-size: 0.875rem;">
      JavaScript is disabled. Please select your preferred language above.<br />
      检测到 JavaScript 未启用，请点击上方按钮进入对应语言文档。
    </p>
  </noscript>
</div>
