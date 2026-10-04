use chrono::Local;

use super::{FlushRequest, VaultConvention};

/// Synthesizes complete Markdown note content based on the flush request and detected vault convention.
pub fn synthesize_markdown(request: &FlushRequest, convention: &VaultConvention) -> String {
    let today_str = Local::now().date_naive().format("%Y-%m-%d").to_string();

    if let Some(ref tmpl) = convention.template {
        // Substitute into existing template
        substitute_template(tmpl, request, &today_str, convention.has_frontmatter)
    } else {
        // Standard structured format
        build_standard_markdown(request, &today_str, convention.has_frontmatter)
    }
}

/// Substitute request fields into a template string.
fn substitute_template(
    template: &str,
    request: &FlushRequest,
    date_str: &str,
    has_frontmatter: bool,
) -> String {
    let tags_comma = request
        .tags
        .iter()
        .map(|t| t.trim_start_matches('#').to_string())
        .collect::<Vec<_>>()
        .join(", ");

    let related_wikilinks = request
        .related_notes
        .iter()
        .map(|n| {
            let t = n.trim();
            if t.starts_with("[[") && t.ends_with("]]") {
                t.to_string()
            } else {
                format!("[[{}]]", t)
            }
        })
        .collect::<Vec<_>>()
        .join(", ");

    let summary = request.summary.as_deref().unwrap_or("");

    let mut rendered = template.to_string();

    // Variable substitutions (case-tolerant)
    let replacements = [
        ("{{title}}", request.title.as_str()),
        ("{{Title}}", request.title.as_str()),
        ("{{summary}}", summary),
        ("{{Summary}}", summary),
        ("{{content}}", request.content.as_str()),
        ("{{Content}}", request.content.as_str()),
        ("{{date}}", date_str),
        ("{{Date}}", date_str),
        ("{{category}}", request.category.as_str()),
        ("{{Category}}", request.category.as_str()),
        ("{{tags}}", tags_comma.as_str()),
        ("{{Tags}}", tags_comma.as_str()),
        ("{{related}}", related_wikilinks.as_str()),
        ("{{Related}}", related_wikilinks.as_str()),
        ("{{related_notes}}", related_wikilinks.as_str()),
        ("{{RelatedNotes}}", related_wikilinks.as_str()),
    ];

    for (needle, val) in replacements {
        rendered = rendered.replace(needle, val);
    }

    // If template didn't include {{related}} or {{related_notes}} and there are related notes, append section
    if !request.related_notes.is_empty()
        && !template.contains("{{related")
        && !rendered.contains("## Related Notes")
    {
        if !rendered.ends_with('\n') {
            rendered.push('\n');
        }
        rendered.push('\n');
        rendered.push_str(&format_related_notes_section(&request.related_notes));
    }

    // Prepend frontmatter if required and template doesn't already have one
    let starts_with_fm = rendered.trim_start().starts_with("---");
    if has_frontmatter && !starts_with_fm {
        let fm = build_yaml_frontmatter(request, date_str);
        format!("{}{}", fm, rendered)
    } else {
        rendered
    }
}

/// Build standard, well-formed markdown note.
fn build_standard_markdown(
    request: &FlushRequest,
    date_str: &str,
    has_frontmatter: bool,
) -> String {
    let mut doc = String::new();

    if has_frontmatter {
        doc.push_str(&build_yaml_frontmatter(request, date_str));
    }

    // Note Title
    doc.push_str(&format!("# {}\n\n", request.title.trim()));

    // Optional Summary callout block
    if let Some(ref sum) = request.summary {
        let trimmed_sum = sum.trim();
        if !trimmed_sum.is_empty() {
            doc.push_str(&format!("> **Summary**: {}\n\n", trimmed_sum));
        }
    }

    // Main content
    let trimmed_content = request.content.trim();
    if trimmed_content.starts_with('#') {
        doc.push_str(trimmed_content);
        doc.push('\n');
    } else {
        doc.push_str("## Context & Decisions\n\n");
        doc.push_str(trimmed_content);
        doc.push('\n');
    }

    // Related Notes section
    if !request.related_notes.is_empty() {
        doc.push('\n');
        doc.push_str(&format_related_notes_section(&request.related_notes));
    }

    doc
}

/// Format YAML Frontmatter block.
fn build_yaml_frontmatter(request: &FlushRequest, date_str: &str) -> String {
    let mut fm = String::from("---\n");
    let safe_title = request.title.replace('"', "\\\"");
    fm.push_str(&format!("title: \"{}\"\n", safe_title));
    fm.push_str(&format!("date: {}\n", date_str));
    fm.push_str(&format!("category: {}\n", request.category.to_lowercase()));

    if request.tags.is_empty() {
        fm.push_str("tags: []\n");
    } else {
        fm.push_str("tags:\n");
        for tag in &request.tags {
            let clean = tag.trim().trim_start_matches('#');
            if !clean.is_empty() {
                fm.push_str(&format!("  - {}\n", clean));
            }
        }
    }

    fm.push_str("---\n\n");
    fm
}

/// Format bulleted [[WikiLinks]] section.
fn format_related_notes_section(notes: &[String]) -> String {
    let mut sec = String::from("## Related Notes\n");
    for note in notes {
        let trimmed = note.trim();
        if trimmed.starts_with("[[") && trimmed.ends_with("]]") {
            sec.push_str(&format!("- {}\n", trimmed));
        } else {
            sec.push_str(&format!("- [[{}]]\n", trimmed));
        }
    }
    sec
}
