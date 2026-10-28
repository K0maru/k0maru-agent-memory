import { defineConfig } from 'vitepress'

const jsonLd = {
  '@context': 'https://schema.org',
  '@graph': [
    {
      '@type': 'Organization',
      '@id': 'https://k0maru.github.io/k0maru-agent-memory/#organization',
      name: 'K0maru Agent Memory',
      url: 'https://k0maru.github.io/k0maru-agent-memory/en/',
      logo: 'https://k0maru.github.io/k0maru-agent-memory/images/architecture_overview.png',
      sameAs: [
        'https://github.com/K0maru/k0maru-agent-memory',
        'https://github.com/K0maru/homebrew-tap',
        'https://en.wikipedia.org/wiki/Model_Context_Protocol',
        'https://www.wikidata.org/wiki/Q116967633',
        'https://www.linkedin.com/company/github',
        'https://www.crunchbase.com/organization/github',
      ],
      contactPoint: {
        '@type': 'ContactPoint',
        contactType: 'technical support',
        url: 'https://github.com/K0maru/k0maru-agent-memory/issues',
        email: 'noreply@github.com',
      },
    },
    {
      '@type': 'WebSite',
      '@id': 'https://k0maru.github.io/k0maru-agent-memory/#website',
      name: 'K0maru Agent Memory',
      url: 'https://k0maru.github.io/k0maru-agent-memory/en/',
      datePublished: '2026-10-01',
      dateModified: '2026-10-28',
      author: {
        '@id': 'https://k0maru.github.io/k0maru-agent-memory/#organization',
      },
      publisher: {
        '@id': 'https://k0maru.github.io/k0maru-agent-memory/#organization',
      },
      potentialAction: {
        '@type': 'SearchAction',
        target: 'https://k0maru.github.io/k0maru-agent-memory/en/?q={search_term_string}',
        'query-input': 'required name=search_term_string',
      },
    },
    {
      '@type': 'SoftwareApplication',
      name: 'K0maru Agent Memory',
      applicationCategory: 'DeveloperApplication',
      operatingSystem: 'macOS, Linux, Windows',
      softwareVersion: '1.0.0',
      datePublished: '2026-10-01',
      dateModified: '2026-10-28',
      license: 'https://opensource.org/licenses/MIT',
      url: 'https://github.com/K0maru/k0maru-agent-memory',
      downloadUrl: 'https://github.com/K0maru/k0maru-agent-memory/releases/latest',
      author: {
        '@id': 'https://k0maru.github.io/k0maru-agent-memory/#organization',
      },
      offers: {
        '@type': 'Offer',
        price: '0',
        priceCurrency: 'USD',
      },
    },
    {
      '@type': 'FAQPage',
      mainEntity: [
        {
          '@type': 'Question',
          name: 'What is K0maru Agent Memory?',
          acceptedAnswer: {
            '@type': 'Answer',
            text: 'K0maru Agent Memory is a single-static-binary (3.6 MB pure-Rust core), zero-daemon long-term memory hub and sub-10ms Living ADR Guard for AI coding agents such as Claude Code, Cursor, Windsurf, Cline, and Gemini CLI. It converts human-authored Markdown Architecture Decision Records into deterministic Git pre-commit and Claude Code Stop-hook guardrails while maintaining bounded <300 token loadouts.',
          },
        },
        {
          '@type': 'Question',
          name: 'How does K0maru Agent Memory compare to Mem0, Zep, and ai-memory?',
          acceptedAnswer: {
            '@type': 'Answer',
            text: 'Unlike passive session loggers (Mem0, Zep, ai-memory) that store raw chat histories and cause context poisoning and belief entrenchment, K0maru provides active, deterministic architectural enforcement (<10ms Living ADR Guard), zero-LLM supersession (k0maru flush --supersedes), a strict <300 token loadout budget, zero daemon background processes, and a 3.4ms cold start.',
          },
        },
        {
          '@type': 'Question',
          name: 'How does the sub-10ms Living ADR Guard prevent coding agents from violating architectural rules?',
          acceptedAnswer: {
            '@type': 'Answer',
            text: 'The Living ADR Guard extracts declarative K-DSL invariant rules directly from Markdown ADRs. It executes AST-aware and diff-aware pattern checks in <10ms in-memory during git pre-commit hooks and Claude Code Stop-hooks (hooks.Stop), halting non-compliant code before it is committed or handed back to humans with actionable remediation feedback.',
          },
        },
        {
          '@type': 'Question',
          name: 'How does deterministic ADR supersession (k0maru flush --supersedes) prevent belief entrenchment?',
          acceptedAnswer: {
            '@type': 'Answer',
            text: 'When architectural decisions evolve, k0maru flush --supersedes atomically sets frontmatter status: superseded on the deprecated ADR, strips inbound WikiLink graph boosts, hot-swaps active guard rules, and activates the new invariant. This eliminates stale belief entrenchment deterministically without LLM re-indexing or manual wiki cleanup.',
          },
        },
        {
          '@type': 'Question',
          name: 'Do I need an existing Obsidian vault to use K0maru Agent Memory?',
          acceptedAnswer: {
            '@type': 'Answer',
            text: 'No. Running k0maru init automatically creates a self-contained .k0maru/wiki/ repository-local Markdown knowledge base with standard 00_Meta, 01_AI_Logs, and 02_Architecture directories. If you already use an Obsidian vault, you can seamlessly bind it via k0maru init --vault <path>.',
          },
        },
        {
          '@type': 'Question',
          name: 'How do I install K0maru Agent Memory in Claude Code, Cursor, and Windsurf?',
          acceptedAnswer: {
            '@type': 'Answer',
            text: 'Run curl -fsSL https://raw.githubusercontent.com/K0maru/k0maru-agent-memory/main/install.sh | bash or brew install K0maru/tap/k0maru. Then run k0maru init --install in your project root, which automatically generates .mcp.json for Claude Code and .cursor/mcp.json for Cursor, configures pre-commit hooks, and registers FastMCP tools.',
          },
        },
      ],
    },
  ],
}

export default defineConfig({
  title: 'K0maru Agent Memory',
  description: 'A Standalone, Single-Static-Binary, Zero-Daemon Long-Term Memory Hub for AI Coding Agents',
  base: process.env.VITEPRESS_BASE || '/',
  sitemap: {
    hostname: 'https://k0maru.github.io/k0maru-agent-memory/',
  },
  appearance: 'force-dark',
  cleanUrls: true,
  lastUpdated: true,
  srcExclude: ['agents/**'],
  transformHtml(code) {
    return code
      .replace(/[\u200B\uFE0F]/g, '')
      .replace(/&#8203;/g, '')
  },

  head: [
    ['meta', { name: 'theme-color', content: '#0F172A' }],
    ['meta', { name: 'og:type', content: 'website' }],
    ['meta', { name: 'og:site_name', content: 'K0maru Agent Memory Documentation' }],
    ['meta', { name: 'author', content: 'K0maru Contributors' }],
    ['meta', { name: 'date', content: '2026-10-28' }],
    ['meta', { property: 'article:published_time', content: '2026-10-01T00:00:00Z' }],
    ['meta', { property: 'article:modified_time', content: '2026-10-28T00:00:00Z' }],
    ['meta', { property: 'og:title', content: 'K0maru Agent Memory — Executable Architectural Memory & Living ADR Guard for AI Coding Agents' }],
    ['meta', { property: 'og:description', content: 'Single-static-binary (3.6 MB Rust), zero-daemon long-term memory hub and <10ms Living ADR Guard for Claude Code, Cursor, Windsurf, and Gemini CLI.' }],
    ['meta', { property: 'og:url', content: 'https://k0maru.github.io/k0maru-agent-memory/en/' }],
    ['meta', { property: 'og:image', content: 'https://k0maru.github.io/k0maru-agent-memory/images/architecture_overview.png' }],
    ['meta', { name: 'twitter:card', content: 'summary_large_image' }],
    ['link', { rel: 'canonical', href: 'https://k0maru.github.io/k0maru-agent-memory/en/' }],
    ['link', { rel: 'alternate', type: 'application/rss+xml', title: 'K0maru Agent Memory Releases Atom Feed', href: 'https://github.com/K0maru/k0maru-agent-memory/releases.atom' }],
    ['script', { type: 'application/ld+json' }, JSON.stringify(jsonLd)],
  ],
  locales: {
    zh: {
      label: '简体中文',
      lang: 'zh-CN',
      link: '/zh/',
      title: 'K0maru Agent Memory',
      description: 'AI 编码智能体单二进制、零常驻轻量级长期记忆核心',
      themeConfig: {
        nav: [
          { text: '起步指南', link: '/zh/guide/introduction' },
          { text: '架构与内核', link: '/zh/architecture/overview' },
          { text: '生态与场景', link: '/zh/workflows/ecosystem' },
          { text: '参考手册', link: '/zh/reference/cli' },
          { text: '实测看板', link: '/zh/benchmarks/a100-evaluation' },
          { text: '路线图', link: '/zh/architecture/roadmap' },
        ],
        sidebar: [
          {
            text: '起步指南',
            items: [
              { text: '项目介绍与设计哲学', link: '/zh/guide/introduction' },
              { text: '安装与环境配置', link: '/zh/guide/installation' },
              { text: '3 分钟极速起步', link: '/zh/guide/quickstart' },
            ],
          },
          {
            text: '架构与内核',
            items: [
              { text: '四层架构与核心数据流', link: '/zh/architecture/overview' },
              { text: '核心算法与关键模块深度实现', link: '/zh/architecture/key-implementations' },
              { text: '演进路线图与架构不变量', link: '/zh/architecture/roadmap' },
            ],
          },
          {
            text: '生态与场景',
            items: [
              { text: '智能体生态挂载 (Claude, Cursor, etc.)', link: '/zh/workflows/ecosystem' },
              { text: '工业级应用场景实战', link: '/zh/workflows/scenarios' },
            ],
          },
          {
            text: '参考手册',
            items: [
              { text: 'CLI 完整命令与参数手册', link: '/zh/reference/cli' },
              { text: 'FastMCP 协议规范与工具契约', link: '/zh/reference/mcp' },
            ],
          },
          {
            text: '实测看板',
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
      title: 'K0maru Agent Memory',
      description: 'A Standalone, Single-Static-Binary, Zero-Daemon Long-Term Memory Hub for AI Coding Agents',
      themeConfig: {
        nav: [
          { text: 'Guide', link: '/en/guide/introduction' },
          { text: 'Architecture', link: '/en/architecture/overview' },
          { text: 'Workflows', link: '/en/workflows/ecosystem' },
          { text: 'Reference', link: '/en/reference/cli' },
          { text: 'Benchmarks', link: '/en/benchmarks/a100-evaluation' },
          { text: 'Roadmap', link: '/en/architecture/roadmap' },
        ],
        sidebar: [
          {
            text: 'Getting Started',
            items: [
              { text: 'Introduction & Philosophy', link: '/en/guide/introduction' },
              { text: 'Installation Guide', link: '/en/guide/installation' },
              { text: '3-Minute Quickstart', link: '/en/guide/quickstart' },
            ],
          },
          {
            text: 'Architecture & Core',
            items: [
              { text: 'Architectural Overview & Data Flow', link: '/en/architecture/overview' },
              { text: 'Key Implementations Deep Dive', link: '/en/architecture/key-implementations' },
              { text: 'Roadmap & Future Invariants', link: '/en/architecture/roadmap' },
            ],
          },
          {
            text: 'Ecosystem & Scenarios',
            items: [
              { text: 'Agent Ecosystem Integration', link: '/en/workflows/ecosystem' },
              { text: 'Industrial Production Scenarios', link: '/en/workflows/scenarios' },
            ],
          },
          {
            text: 'Reference Manual',
            items: [
              { text: 'CLI Subcommands & Flags', link: '/en/reference/cli' },
              { text: 'FastMCP Stdio Protocol Contracts', link: '/en/reference/mcp' },
            ],
          },
          {
            text: 'Benchmarks',
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
