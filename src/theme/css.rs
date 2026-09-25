use super::model::{Theme, FILE_TOKENS};
use std::fmt::Write;

// ═══════════════════════════════════════════════
//  Theme → CSS
// ═══════════════════════════════════════════════

const BASE_TEMPLATE: &str = include_str!("base.css");

/// Full application stylesheet for `theme`.
pub fn generate(theme: &Theme) -> Result<String, String> {
    let mut css = adwaita_overrides(theme);
    css.push_str(&render(BASE_TEMPLATE, theme)?);
    css.push_str("\n/* ── File kind colors (tree view names) ── */\n");
    for kind in FILE_TOKENS {
        let color = theme
            .color(&format!("file-{}", kind))
            .expect("resolved theme is complete");
        let _ = writeln!(css, ".tree-kind-{} {{ color: {}; }}", kind, color);
    }
    Ok(css)
}

/// Re-points libadwaita's own colors at the theme, so stock widgets
/// (dialogs, popovers, switches…) match. Emitted twice: as named colors
/// for libadwaita < 1.6 and as CSS variables for 1.6+.
fn adwaita_overrides(theme: &Theme) -> String {
    let c = |key: &str| {
        theme
            .color(key)
            .expect("resolved theme is complete")
            .to_string()
    };
    let pairs: [(&str, String); 21] = [
        ("window_bg_color", c("window")),
        ("window_fg_color", c("text")),
        ("view_bg_color", c("window")),
        ("view_fg_color", c("text")),
        ("headerbar_bg_color", c("sidebar")),
        ("headerbar_fg_color", c("text")),
        ("headerbar_backdrop_color", c("sidebar")),
        ("sidebar_bg_color", c("sidebar")),
        ("sidebar_fg_color", c("text")),
        ("popover_bg_color", c("window")),
        ("popover_fg_color", c("text")),
        ("dialog_bg_color", c("window")),
        ("dialog_fg_color", c("text")),
        ("card_bg_color", c("surface")),
        ("card_fg_color", c("text")),
        ("accent_bg_color", c("accent")),
        ("accent_fg_color", c("accent-text")),
        ("accent_color", c("accent")),
        ("destructive_bg_color", c("danger")),
        ("destructive_fg_color", c("accent-text")),
        ("destructive_color", c("danger")),
    ];

    let mut css = String::from("/* ── libadwaita color overrides ── */\n");
    for (name, value) in &pairs {
        let _ = writeln!(css, "@define-color {} {};", name, value);
    }
    css.push_str(":root {\n");
    for (name, value) in &pairs {
        let _ = writeln!(css, "    --{}: {};", name.replace('_', "-"), value);
    }
    css.push_str("}\n\n");
    css
}

/// Expands `{{…}}` placeholders (see the header of `base.css`).
fn render(template: &str, theme: &Theme) -> Result<String, String> {
    let mut out = String::with_capacity(template.len() + 1024);
    let mut rest = template;
    while let Some(start) = rest.find("{{") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        let end = after.find("}}").ok_or("unclosed {{ in base.css template")?;
        out.push_str(&expand(after[..end].trim(), theme)?);
        rest = &after[end + 2..];
    }
    out.push_str(rest);
    Ok(out)
}

fn expand(placeholder: &str, theme: &Theme) -> Result<String, String> {
    let px = |v: f64| format!("{}px", (v * 10.0).round() / 10.0);
    let number = |arg: &str| {
        arg.parse::<f64>()
            .map_err(|_| format!("bad number in {{{{{}}}}}", placeholder))
    };

    if let Some((func, arg)) = placeholder.split_once(' ') {
        let n = number(arg.trim())?;
        return match func {
            "radius" => Ok(px((theme.radius * n).round())),
            "sp" => Ok(px((n * theme.density.factor()).round())),
            "fs" => Ok(px(n * theme.font_scale)),
            _ => Err(format!(
                "unknown template function in {{{{{}}}}}",
                placeholder
            )),
        };
    }
    match placeholder {
        // No rule at all for "system", so GTK keeps the desktop's font.
        "font-ui-rule" => {
            return Ok(match &theme.font_ui {
                Some(family) => format!("font-family: {};", family),
                None => String::new(),
            })
        }
        "font-mono" => return Ok(theme.font_mono.clone()),
        _ => {}
    }
    let (name, alpha) = match placeholder.split_once('/') {
        Some((name, a)) => (name, Some(number(a)?)),
        None => (placeholder, None),
    };
    let color = theme
        .color(name)
        .ok_or_else(|| format!("unknown color {{{{{}}}}} in base.css", name))?;
    Ok(match alpha {
        Some(a) => color.with_alpha(a).to_string(),
        None => color.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::model::{Presets, ThemeFile, BUILTIN_PRESETS};

    fn theme(src: &str) -> Theme {
        Theme::resolve(&ThemeFile::parse(src).unwrap(), &Presets { user_dir: None }).unwrap()
    }

    #[test]
    fn every_preset_renders_without_leftover_placeholders() {
        for (id, _) in BUILTIN_PRESETS {
            let css = generate(&theme(&format!("base = \"{id}\""))).unwrap();
            assert!(
                !css.contains("{{") && !css.contains("}}"),
                "{id}: leftover placeholder"
            );
            assert!(css.contains(".tree-kind-rust"), "{id}");
        }
    }

    #[test]
    fn tokens_flow_into_css() {
        let css = generate(&theme(
            r##"
            [colors]
            accent = "#ff0000"
            [shape]
            radius = 20
            density = "spacious"
            [fonts]
            ui = "Figtree"
            scale = 1.5
            "##,
        ))
        .unwrap();
        assert!(css.contains("@define-color accent_bg_color #ff0000;"));
        assert!(css.contains("--accent-bg-color: #ff0000;"));
        assert!(
            css.contains("rgba(255, 0, 0, 0.14)"),
            "derived selection color"
        );
        assert!(css.contains("font-family: Figtree;"));
        // Presets default to the desktop font: no font-family rule.
        assert!(!generate(&theme("")).unwrap().contains("font-family"));
        // .file-card: radius 1.0 × 20, padding 14 × 1.25, name 12 × 1.5
        assert!(css.contains("border-radius: 20px;"));
        assert!(css.contains("padding: 18px"));
        assert!(css.contains("font-size: 18px;"));
    }

    #[test]
    fn render_reports_bad_placeholders() {
        let t = theme("");
        assert!(render("{{nope}}", &t)
            .unwrap_err()
            .contains("unknown color"));
        assert!(render("{{sp x}}", &t).unwrap_err().contains("bad number"));
        assert!(render("{{accent", &t).unwrap_err().contains("unclosed"));
        assert_eq!(render("a {{accent}} b", &t).unwrap(), "a #89b4fa b");
    }
}
