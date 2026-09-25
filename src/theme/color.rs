use std::fmt;

// ─── Color Values ───
//
// Theme colors are parsed into RGBA so the CSS generator can derive
// translucent variants (`{{accent/0.14}}`) from any user-supplied color.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rgba {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    /// 0.0 – 1.0
    pub a: f64,
}

impl Rgba {
    /// Parses `#rgb`, `#rgba`, `#rrggbb`, `#rrggbbaa`, `rgb(r, g, b)`,
    /// `rgba(r, g, b, a)` and `transparent`.
    pub fn parse(input: &str) -> Result<Rgba, String> {
        let s = input.trim();
        if s.eq_ignore_ascii_case("transparent") {
            return Ok(Rgba {
                r: 0,
                g: 0,
                b: 0,
                a: 0.0,
            });
        }
        if let Some(hex) = s.strip_prefix('#') {
            return parse_hex(hex).ok_or_else(|| format!("invalid hex color “{}”", input));
        }
        if let Some(args) = s
            .strip_prefix("rgba(")
            .or_else(|| s.strip_prefix("rgb("))
            .and_then(|rest| rest.strip_suffix(')'))
        {
            return parse_rgb_fn(args).ok_or_else(|| format!("invalid rgb() color “{}”", input));
        }
        Err(format!(
            "unsupported color “{}” (use #rrggbb, rgb(), rgba() or @name)",
            input
        ))
    }

    /// Same color with its alpha multiplied by `alpha`.
    pub fn with_alpha(self, alpha: f64) -> Rgba {
        Rgba {
            a: (self.a * alpha).clamp(0.0, 1.0),
            ..self
        }
    }
}

impl fmt::Display for Rgba {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.a >= 1.0 {
            write!(f, "#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
        } else {
            // Trim float noise: 0.14 not 0.14000000000000001
            let a = (self.a * 1000.0).round() / 1000.0;
            write!(f, "rgba({}, {}, {}, {})", self.r, self.g, self.b, a)
        }
    }
}

fn parse_hex(hex: &str) -> Option<Rgba> {
    if !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let digit = |i: usize| u8::from_str_radix(&hex[i..i + 1], 16).ok().map(|v| v * 17);
    let pair = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).ok();
    let (r, g, b, a) = match hex.len() {
        3 => (digit(0)?, digit(1)?, digit(2)?, 255),
        4 => (digit(0)?, digit(1)?, digit(2)?, digit(3)?),
        6 => (pair(0)?, pair(2)?, pair(4)?, 255),
        8 => (pair(0)?, pair(2)?, pair(4)?, pair(6)?),
        _ => return None,
    };
    Some(Rgba {
        r,
        g,
        b,
        a: a as f64 / 255.0,
    })
}

fn parse_rgb_fn(args: &str) -> Option<Rgba> {
    let parts: Vec<&str> = args.split(',').map(str::trim).collect();
    if parts.len() != 3 && parts.len() != 4 {
        return None;
    }
    let channel = |s: &str| s.parse::<u8>().ok();
    let a = match parts.get(3) {
        Some(s) => s.parse::<f64>().ok().filter(|a| (0.0..=1.0).contains(a))?,
        None => 1.0,
    };
    Some(Rgba {
        r: channel(parts[0])?,
        g: channel(parts[1])?,
        b: channel(parts[2])?,
        a,
    })
}

#[cfg(test)]
mod tests {
    use super::Rgba;

    #[test]
    fn parses_hex_forms() {
        assert_eq!(Rgba::parse("#fff").unwrap().to_string(), "#ffffff");
        assert_eq!(Rgba::parse("#1e1e2e").unwrap().to_string(), "#1e1e2e");
        assert_eq!(Rgba::parse("#1E1E2E").unwrap().to_string(), "#1e1e2e");
        assert_eq!(
            Rgba::parse("#00000080").unwrap().to_string(),
            "rgba(0, 0, 0, 0.502)"
        );
    }

    #[test]
    fn parses_rgb_functions() {
        let c = Rgba::parse("rgba(205, 214, 244, 0.06)").unwrap();
        assert_eq!((c.r, c.g, c.b), (205, 214, 244));
        assert_eq!(c.to_string(), "rgba(205, 214, 244, 0.06)");
        assert_eq!(Rgba::parse("rgb(1,2,3)").unwrap().to_string(), "#010203");
    }

    #[test]
    fn derives_alpha() {
        let accent = Rgba::parse("#89b4fa").unwrap();
        assert_eq!(
            accent.with_alpha(0.14).to_string(),
            "rgba(137, 180, 250, 0.14)"
        );
    }

    #[test]
    fn rejects_garbage() {
        for bad in [
            "",
            "#12",
            "#ggg",
            "red",
            "rgb(1,2)",
            "rgba(1,2,3,4)",
            "#fff; } * {",
        ] {
            assert!(Rgba::parse(bad).is_err(), "{bad:?} should be rejected");
        }
    }
}
