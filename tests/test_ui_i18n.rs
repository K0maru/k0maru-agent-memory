use k0maru::ui::routes::DashboardAssets;

#[test]
fn test_embedded_index_html_i18n_attributes_and_switcher() {
    let index_file = DashboardAssets::get("index.html").expect("index.html must exist");
    let html = std::str::from_utf8(&index_file.data).expect("valid utf-8 html");

    // Header Language switcher toggle button & indicator
    assert!(
        html.contains("id=\"btn-lang-toggle\""),
        "index.html must contain #btn-lang-toggle"
    );
    assert!(
        html.contains("btn-lang"),
        "index.html must contain .btn-lang class"
    );
    assert!(
        html.contains("id=\"lang-indicator\""),
        "index.html must contain #lang-indicator"
    );

    // i18n attributes for static DOM translation
    assert!(
        html.contains("data-i18n="),
        "index.html must contain data-i18n attributes"
    );
    assert!(
        html.contains("data-i18n-placeholder="),
        "index.html must contain data-i18n-placeholder attributes"
    );
    assert!(
        html.contains("data-i18n-title="),
        "index.html must contain data-i18n-title attributes"
    );

    // Brand and Status
    assert!(
        html.contains("data-i18n=\"brand.subtitle\""),
        "index.html must have data-i18n for brand.subtitle"
    );
    assert!(
        html.contains("data-i18n-title=\"status.vault_title\""),
        "index.html must have status.vault_title"
    );
    assert!(
        html.contains("data-i18n=\"status.vault\""),
        "index.html must have status.vault"
    );

    // Navigation Tabs
    assert!(
        html.contains("data-i18n=\"tabs.search\""),
        "index.html must have data-i18n for tabs.search"
    );
    assert!(
        html.contains("data-i18n=\"tabs.graph\""),
        "index.html must have data-i18n for tabs.graph"
    );
    assert!(
        html.contains("data-i18n=\"tabs.logs\""),
        "index.html must have data-i18n for tabs.logs"
    );
    assert!(
        html.contains("data-i18n=\"tabs.scoreboard\""),
        "index.html must have data-i18n for tabs.scoreboard"
    );

    // Panel 1: Search Debugger
    assert!(
        html.contains("data-i18n=\"search.title\""),
        "index.html must have data-i18n for search.title"
    );
    assert!(
        html.contains("data-i18n-placeholder=\"search.placeholder\""),
        "index.html must have search placeholder i18n"
    );
    assert!(
        html.contains("data-i18n=\"search.mode_hybrid\""),
        "index.html must have data-i18n for search.mode_hybrid"
    );

    // Panel 2: Graph Explorer
    assert!(
        html.contains("data-i18n=\"graph.title\""),
        "index.html must have data-i18n for graph.title"
    );
    assert!(
        html.contains("data-i18n-placeholder=\"graph.search_placeholder\""),
        "index.html must have graph search placeholder"
    );
    assert!(
        html.contains("data-i18n=\"graph.tier_project\""),
        "index.html must have tier_project"
    );
    assert!(
        html.contains("data-i18n=\"graph.highlight_orphans\""),
        "index.html must have highlight_orphans"
    );

    // Panel 3: Log Inspector
    assert!(
        html.contains("data-i18n=\"logs.title\""),
        "index.html must have data-i18n for logs.title"
    );
    assert!(
        html.contains("data-i18n-placeholder=\"logs.search_placeholder\""),
        "index.html must have logs search placeholder"
    );
    assert!(
        html.contains("data-i18n=\"logs.copy_node_id\""),
        "index.html must have copy_node_id"
    );
    assert!(
        html.contains("data-i18n=\"logs.copy_raw_log\""),
        "index.html must have copy_raw_log"
    );

    // Panel 4: Token Scoreboard
    assert!(
        html.contains("data-i18n=\"scoreboard.title\""),
        "index.html must have data-i18n for scoreboard.title"
    );
    assert!(
        html.contains("data-i18n=\"scoreboard.token_title\""),
        "index.html must have token_title"
    );
    assert!(
        html.contains("data-i18n=\"scoreboard.kpi_docs_label\""),
        "index.html must have kpi_docs_label"
    );
}

#[test]
fn test_embedded_app_js_i18n_engine_and_dictionaries() {
    let app_file = DashboardAssets::get("app.js").expect("app.js must exist");
    let js = std::str::from_utf8(&app_file.data).expect("valid utf-8 js");

    // Engine declarations and functions
    assert!(
        js.contains("TRANSLATIONS"),
        "app.js must contain TRANSLATIONS dictionary"
    );
    assert!(
        js.contains("function getLanguage")
            || js.contains("getLanguage =")
            || js.contains("const getLanguage"),
        "app.js must define getLanguage()"
    );
    assert!(
        js.contains("function setLanguage")
            || js.contains("setLanguage =")
            || js.contains("const setLanguage"),
        "app.js must define setLanguage()"
    );
    assert!(
        js.contains("k0maru_lang"),
        "app.js must persist language under k0maru_lang in localStorage"
    );
    assert!(
        js.contains("navigator.language"),
        "app.js must inspect navigator.language for default fallback"
    );
    assert!(
        js.contains("btn-lang-toggle"),
        "app.js must attach event listener to btn-lang-toggle"
    );
    assert!(
        js.contains("document.documentElement.lang"),
        "app.js must set document.documentElement.lang"
    );

    // Bilingual translation strings verification
    // Tabs
    assert!(
        js.contains("Search Debugger"),
        "app.js must contain Search Debugger"
    );
    assert!(
        js.contains("多路检索调试"),
        "app.js must contain 多路检索调试"
    );
    assert!(
        js.contains("Graph Explorer"),
        "app.js must contain Graph Explorer"
    );
    assert!(
        js.contains("知识图谱图鉴"),
        "app.js must contain 知识图谱图鉴"
    );
    assert!(
        js.contains("Log Inspector"),
        "app.js must contain Log Inspector"
    );
    assert!(
        js.contains("日志与状态机透视"),
        "app.js must contain 日志与状态机透视"
    );
    assert!(
        js.contains("Token Scoreboard"),
        "app.js must contain Token Scoreboard"
    );
    assert!(
        js.contains("Token 节省计分板"),
        "app.js must contain Token 节省计分板"
    );

    // Header & Sync
    assert!(js.contains("Sync Vault"), "app.js must contain Sync Vault");
    assert!(js.contains("同步知识库"), "app.js must contain 同步知识库");
    assert!(js.contains("Syncing..."), "app.js must contain Syncing...");
    assert!(js.contains("同步中..."), "app.js must contain 同步中...");
    assert!(js.contains("Synced"), "app.js must contain Synced");
    assert!(js.contains("已同步"), "app.js must contain 已同步");

    // Badges & Explainability
    assert!(
        js.contains("Graph Boost"),
        "app.js must contain Graph Boost"
    );
    assert!(js.contains("图谱加权"), "app.js must contain 图谱加权");
    assert!(
        js.contains("Highlight Orphans"),
        "app.js must contain Highlight Orphans"
    );
    assert!(
        js.contains("高亮孤立笔记"),
        "app.js must contain 高亮孤立笔记"
    );

    // Copy actions
    assert!(
        js.contains("Copy Node ID"),
        "app.js must contain Copy Node ID"
    );
    assert!(
        js.contains("复制 Node ID"),
        "app.js must contain 复制 Node ID"
    );
    assert!(
        js.contains("Copy Raw Log"),
        "app.js must contain Copy Raw Log"
    );
    assert!(
        js.contains("复制原始日志"),
        "app.js must contain 复制原始日志"
    );
    assert!(js.contains("Copied!"), "app.js must contain Copied!");
    assert!(js.contains("已复制!"), "app.js must contain 已复制!");

    // Hierarchy tiers
    assert!(
        js.contains("L3 Evergreen"),
        "app.js must contain L3 Evergreen"
    );
    assert!(
        js.contains("L3 永恒笔记"),
        "app.js must contain L3 永恒笔记"
    );
    assert!(js.contains("L2 Log"), "app.js must contain L2 Log");
    assert!(
        js.contains("L2 决策日志"),
        "app.js must contain L2 决策日志"
    );
    assert!(
        js.contains("L1 Resource"),
        "app.js must contain L1 Resource"
    );
    assert!(
        js.contains("L1 资源引用"),
        "app.js must contain L1 资源引用"
    );
    assert!(js.contains("L0 Daily"), "app.js must contain L0 Daily");
    assert!(js.contains("L0 日记"), "app.js must contain L0 日记");

    // Token Scoreboard Metrics
    assert!(
        js.contains("Token Economics & Context Hygiene") || js.contains("Token Economics"),
        "app.js must contain Token Economics"
    );
    assert!(
        js.contains("Token 经济学与上下文整洁度") || js.contains("Token 经济学"),
        "app.js must contain Token 经济学"
    );
    assert!(
        js.contains("Token Reduction Ratio (TRR)"),
        "app.js must contain Token Reduction Ratio (TRR)"
    );
    assert!(
        js.contains("上下文压缩率 (TRR)"),
        "app.js must contain 上下文压缩率 (TRR)"
    );
}

#[test]
fn test_embedded_style_css_language_switcher_tokens() {
    let style_file = DashboardAssets::get("style.css").expect("style.css must exist");
    let css = std::str::from_utf8(&style_file.data).expect("valid utf-8 css");

    assert!(
        css.contains(".btn-lang"),
        "style.css must contain .btn-lang styles"
    );
    assert!(
        css.contains(".btn-secondary"),
        "style.css must contain .btn-secondary styles"
    );
    assert!(
        css.contains("#lang-indicator"),
        "style.css must contain #lang-indicator styles"
    );
}
