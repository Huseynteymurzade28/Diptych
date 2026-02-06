pub const TOKYO_NIGHT: &str = "
    window { background: #1a1b26; color: #c0caf5; }
    .sidebar { background: #16161e; }
    button { background: #292e42; color: #c0caf5; border: none; }
    button:hover { background: #3b4261; }
    .suggested-action { background: #7aa2f7; color: #15161e; }
    entry { background: #16161e; color: #c0caf5; border: 1px solid #414868; }
    popover { background: #1a1b26; color: #c0caf5; border: 1px solid #414868; }
    listview { background: #1a1b26; }
";

pub const CATPPUCCIN_MOCHA: &str = "
    window { background: #1e1e2e; color: #cdd6f4; }
    .sidebar { background: #181825; }
    button { background: #313244; color: #cdd6f4; border: none; }
    button:hover { background: #45475a; }
    .suggested-action { background: #89b4fa; color: #1e1e2e; }
    entry { background: #181825; color: #cdd6f4; border: 1px solid #313244; }
    popover { background: #1e1e2e; color: #cdd6f4; border: 1px solid #313244; }
";

pub const GRUVBOX_DARK: &str = "
    window { background: #282828; color: #ebdbb2; }
    .sidebar { background: #1d2021; }
    button { background: #3c3836; color: #ebdbb2; border: none; }
    button:hover { background: #504945; }
    .suggested-action { background: #d79921; color: #282828; }
    entry { background: #1d2021; color: #ebdbb2; border: 1px solid #3c3836; }
    popover { background: #282828; color: #ebdbb2; border: 1px solid #3c3836; }
";

pub const NORD: &str = "
    window { background: #2e3440; color: #d8dee9; }
    .sidebar { background: #242933; }
    button { background: #3b4252; color: #d8dee9; border: none; }
    button:hover { background: #434c5e; }
    .suggested-action { background: #88c0d0; color: #2e3440; }
    entry { background: #242933; color: #d8dee9; border: 1px solid #3b4252; }
    popover { background: #2e3440; color: #d8dee9; border: 1px solid #3b4252; }
";

pub const SOLARIZED_DARK: &str = "
    window { background: #002b36; color: #839496; }
    .sidebar { background: #073642; }
    button { background: #073642; color: #839496; border: none; }
    button:hover { background: #586e75; }
    .suggested-action { background: #268bd2; color: #002b36; }
    entry { background: #073642; color: #839496; border: 1px solid #586e75; }
    popover { background: #002b36; color: #839496; border: 1px solid #586e75; }
";

pub fn get_css(name: &str) -> &'static str {
    match name {
        "Tokyo Night" => TOKYO_NIGHT,
        "Catppuccin" => CATPPUCCIN_MOCHA,
        "Gruvbox" => GRUVBOX_DARK,
        "Nord" => NORD,
        "Solarized" => SOLARIZED_DARK,
        _ => TOKYO_NIGHT,
    }
}

pub fn all_themes() -> Vec<&'static str> {
    vec!["Tokyo Night", "Catppuccin", "Gruvbox", "Nord", "Solarized"]
}
