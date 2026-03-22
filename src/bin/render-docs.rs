use std::collections::BTreeMap;
use std::fs;
use std::path::{Component, Path};

use anyhow::{Context, Result, bail};
use regex::Regex;
use rust_i18n_support::load_locales;

#[derive(Clone, Copy)]
enum TemplateKind {
    Readme,
    Guide,
}

#[derive(Clone, Copy)]
struct DocSpec {
    locale: &'static str,
    template: &'static str,
    output: &'static str,
    kind: TemplateKind,
}

const DOCS: [DocSpec; 6] = [
    DocSpec {
        locale: "en",
        template: "docs/templates/readme.md.tpl",
        output: "README.md",
        kind: TemplateKind::Readme,
    },
    DocSpec {
        locale: "zh-CN",
        template: "docs/templates/readme.md.tpl",
        output: "docs/README.zh-CN.md",
        kind: TemplateKind::Readme,
    },
    DocSpec {
        locale: "zh-TW",
        template: "docs/templates/readme.md.tpl",
        output: "docs/README.zh-TW.md",
        kind: TemplateKind::Readme,
    },
    DocSpec {
        locale: "en",
        template: "docs/templates/guide.md.tpl",
        output: "docs/guide.md",
        kind: TemplateKind::Guide,
    },
    DocSpec {
        locale: "zh-CN",
        template: "docs/templates/guide.md.tpl",
        output: "docs/guide.zh-CN.md",
        kind: TemplateKind::Guide,
    },
    DocSpec {
        locale: "zh-TW",
        template: "docs/templates/guide.md.tpl",
        output: "docs/guide.zh-TW.md",
        kind: TemplateKind::Guide,
    },
];

fn main() -> Result<()> {
    let translations = load_locales("locales", |_| false);
    let english = translations
        .get("en")
        .context("missing English translations in locales/")?;

    for spec in DOCS {
        let locale = translations
            .get(spec.locale)
            .with_context(|| format!("missing locale translations for {}", spec.locale))?;
        render_one(spec, locale, english)?;
    }

    Ok(())
}

fn render_one(
    spec: DocSpec,
    locale: &BTreeMap<String, String>,
    fallback: &BTreeMap<String, String>,
) -> Result<()> {
    let template = fs::read_to_string(spec.template)
        .with_context(|| format!("failed to read {}", spec.template))?;
    let extras = build_extras(spec);
    let rendered = render_template(&template, locale, fallback, &extras)?;

    if let Some(parent) = Path::new(spec.output).parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    fs::write(spec.output, rendered).with_context(|| format!("failed to write {}", spec.output))?;
    Ok(())
}

fn build_extras(spec: DocSpec) -> BTreeMap<String, String> {
    let mut extras = BTreeMap::new();
    let output = Path::new(spec.output);

    extras.insert(
        "language_switcher".to_string(),
        build_language_switcher(output, spec.kind),
    );

    match spec.kind {
        TemplateKind::Readme => {
            extras.insert(
                "guide_path".to_string(),
                relative_path(output, Path::new(guide_output(spec.locale))),
            );
            extras.insert(
                "license_path".to_string(),
                relative_path(output, Path::new("LICENSE")),
            );
        }
        TemplateKind::Guide => {}
    }

    extras
}

fn build_language_switcher(output: &Path, kind: TemplateKind) -> String {
    let targets = match kind {
        TemplateKind::Readme => [
            ("English", "README.md"),
            ("简体中文", "docs/README.zh-CN.md"),
            ("繁體中文", "docs/README.zh-TW.md"),
        ],
        TemplateKind::Guide => [
            ("English", "docs/guide.md"),
            ("简体中文", "docs/guide.zh-CN.md"),
            ("繁體中文", "docs/guide.zh-TW.md"),
        ],
    };

    targets
        .into_iter()
        .map(|(label, path)| format!("[{label}]({})", relative_path(output, Path::new(path))))
        .collect::<Vec<_>>()
        .join(" | ")
}

fn guide_output(locale: &str) -> &'static str {
    match locale {
        "en" => "docs/guide.md",
        "zh-CN" => "docs/guide.zh-CN.md",
        "zh-TW" => "docs/guide.zh-TW.md",
        _ => "docs/guide.md",
    }
}

fn relative_path(from_output: &Path, to: &Path) -> String {
    let from_dir = from_output.parent().unwrap_or_else(|| Path::new(""));
    let from_parts = path_parts(from_dir);
    let to_parts = path_parts(to);

    let shared = from_parts
        .iter()
        .zip(to_parts.iter())
        .take_while(|(left, right)| left == right)
        .count();

    let mut parts = Vec::new();
    for _ in shared..from_parts.len() {
        parts.push("..".to_string());
    }
    for part in &to_parts[shared..] {
        parts.push(part.clone());
    }

    if parts.is_empty() {
        ".".to_string()
    } else {
        parts.join("/")
    }
}

fn path_parts(path: &Path) -> Vec<String> {
    path.components()
        .filter_map(|component| match component {
            Component::Normal(part) => Some(part.to_string_lossy().to_string()),
            _ => None,
        })
        .collect()
}

fn render_template(
    template: &str,
    locale: &BTreeMap<String, String>,
    fallback: &BTreeMap<String, String>,
    extras: &BTreeMap<String, String>,
) -> Result<String> {
    let token_re = Regex::new(r"\{\{\s*([A-Za-z0-9._-]+)\s*\}\}")?;
    let mut rendered = template.to_string();

    for _ in 0..8 {
        let next = token_re
            .replace_all(&rendered, |captures: &regex::Captures<'_>| {
                let key = &captures[1];
                extras
                    .get(key)
                    .or_else(|| locale.get(key))
                    .or_else(|| fallback.get(key))
                    .map(|value| value.trim_end_matches('\n').to_string())
                    .unwrap_or_else(|| captures[0].to_string())
            })
            .to_string();

        if next == rendered {
            break;
        }

        rendered = next;
    }

    let unresolved = token_re
        .captures_iter(&rendered)
        .map(|captures| captures[1].to_string())
        .collect::<Vec<_>>();
    if !unresolved.is_empty() {
        bail!("unresolved template tokens: {}", unresolved.join(", "));
    }

    Ok(rendered.trim_end().to_string() + "\n")
}
