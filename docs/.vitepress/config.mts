import { defineConfig } from 'vitepress'

export default defineConfig({
  title: 'K0maru Agent Memory',
  description: 'A Standalone, Single-Static-Binary, Zero-Daemon Long-Term Memory Hub for AI Coding Agents',
  base: process.env.VITEPRESS_BASE || '/',
  appearance: 'force-dark',
  cleanUrls: true,
  lastUpdated: true,

  head: [
    ['meta', { name: 'theme-color', content: '#0F172A' }],
    ['meta', { name: 'og:type', content: 'website' }],
    ['meta', { name: 'og:site_name', content: 'K0maru Agent Memory Documentation' }],
  ],

  locales: {
    zh: {
      label: '简体中文',
      lang: 'zh-CN',
      link: '/zh/',
      title: 'K0maru 架构与技术文档',
      description: 'AI 编码智能体单二进制、零常驻轻量级长期记忆核心',
      themeConfig: {
        nav: [
          { text: '起步指南', link: '/zh/guide/introduction' },
          { text: '架构与内核', link: '/zh/architecture/overview' },
          { text: '生态与场景', link: '/zh/workflows/ecosystem' },
          { text: '参考手册', link: '/zh/reference/cli' },
          { text: '实测看板', link: '/zh/benchmarks/a100-evaluation' },
        ],
        sidebar: [
          {
            text: '🚀 起步指南',
            items: [
              { text: '项目介绍与设计哲学', link: '/zh/guide/introduction' },
              { text: '安装与环境配置', link: '/zh/guide/installation' },
              { text: '3 分钟极速起步', link: '/zh/guide/quickstart' },
            ],
          },
          {
            text: '🏗️ 架构与内核',
            items: [
              { text: '四层架构与核心数据流', link: '/zh/architecture/overview' },
              { text: '核心算法与关键模块深度实现', link: '/zh/architecture/key-implementations' },
            ],
          },
          {
            text: '🤖 生态与场景',
            items: [
              { text: '智能体生态挂载 (Claude, Cursor, etc.)', link: '/zh/workflows/ecosystem' },
              { text: '工业级应用场景实战', link: '/zh/workflows/scenarios' },
            ],
          },
          {
            text: '📖 参考手册',
            items: [
              { text: 'CLI 完整命令与参数手册', link: '/zh/reference/cli' },
              { text: 'FastMCP 协议规范与工具契约', link: '/zh/reference/mcp' },
            ],
          },
          {
            text: '📊 实测看板',
            items: [
              { text: 'A100 实测看板与 Token 经济学', link: '/zh/benchmarks/a100-evaluation' },
            ],
          },
        ],
        outline: {
          level: [2, 3],
          label: '页面大纲',
        },
        docFooter: {
          prev: '上一页',
          next: '下一页',
        },
        lastUpdated: {
          text: '最后更新于',
        },
      },
    },
    en: {
      label: 'English',
      lang: 'en-US',
      link: '/en/',
      title: 'K0maru Architecture & Technical Manual',
      description: 'A Standalone, Single-Static-Binary, Zero-Daemon Long-Term Memory Hub for AI Coding Agents',
      themeConfig: {
        nav: [
          { text: 'Guide', link: '/en/guide/introduction' },
          { text: 'Architecture', link: '/en/architecture/overview' },
          { text: 'Workflows', link: '/en/workflows/ecosystem' },
          { text: 'Reference', link: '/en/reference/cli' },
          { text: 'Benchmarks', link: '/en/benchmarks/a100-evaluation' },
        ],
        sidebar: [
          {
            text: '🚀 Getting Started',
            items: [
              { text: 'Introduction & Philosophy', link: '/en/guide/introduction' },
              { text: 'Installation Guide', link: '/en/guide/installation' },
              { text: '3-Minute Quickstart', link: '/en/guide/quickstart' },
            ],
          },
          {
            text: '🏗️ Architecture & Core',
            items: [
              { text: 'Architectural Overview & Data Flow', link: '/en/architecture/overview' },
              { text: 'Key Implementations Deep Dive', link: '/en/architecture/key-implementations' },
            ],
          },
          {
            text: '🤖 Ecosystem & Scenarios',
            items: [
              { text: 'Agent Ecosystem Integration', link: '/en/workflows/ecosystem' },
              { text: 'Industrial Production Scenarios', link: '/en/workflows/scenarios' },
            ],
          },
          {
            text: '📖 Reference Manual',
            items: [
              { text: 'CLI Subcommands & Flags', link: '/en/reference/cli' },
              { text: 'FastMCP Stdio Protocol Contracts', link: '/en/reference/mcp' },
            ],
          },
          {
            text: '📊 Benchmarks',
            items: [
              { text: 'A100 Evaluation & Token Economics', link: '/en/benchmarks/a100-evaluation' },
            ],
          },
        ],
        outline: {
          level: [2, 3],
          label: 'On this page',
        },
        docFooter: {
          prev: 'Previous page',
          next: 'Next page',
        },
        lastUpdated: {
          text: 'Last updated',
        },
      },
    },
  },

  themeConfig: {
    socialLinks: [
      { icon: 'github', link: 'https://github.com/K0maru/k0maru-agent-memory' },
    ],
    search: {
      provider: 'local',
      options: {
        locales: {
          zh: {
            translations: {
              button: {
                buttonText: '搜索文档',
                buttonAriaLabel: '搜索文档',
              },
              modal: {
                noResultsText: '无法找到相关结果',
                resetButtonTitle: '清除查询条件',
                footer: {
                  selectText: '选择',
                  navigateText: '切换',
                  closeText: '关闭',
                },
              },
            },
          },
          en: {
            translations: {
              button: {
                buttonText: 'Search documentation',
                buttonAriaLabel: 'Search documentation',
              },
              modal: {
                noResultsText: 'No results found',
                resetButtonTitle: 'Reset search',
                footer: {
                  selectText: 'to select',
                  navigateText: 'to navigate',
                  closeText: 'to close',
                },
              },
            },
          },
        },
      },
    },
    footer: {
      message: 'Released under the MIT License.',
      copyright: 'Copyright © 2026 K0maru Contributors',
    },
  },
})
