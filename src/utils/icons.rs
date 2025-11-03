use std::env;

/// Check if Nerd Fonts should be used based on environment variable
pub fn use_nerd_fonts() -> bool {
    env::var("NERD_FONT").unwrap_or_default() == "1"
        || env::var("USE_NERD_FONT").unwrap_or_default() == "1"
}

/// Git-related icons
pub mod git {
    use super::use_nerd_fonts;

    pub fn branch() -> String {
        if use_nerd_fonts() {
            // Nerd Font: nf-dev-git_branch (U+E0A0)
            '\u{e0a0}'.to_string()
        } else {
            "⎇".to_string() // Unicode branch symbol
        }
    }

    #[allow(dead_code)]
    pub fn commit() -> String {
        if use_nerd_fonts() {
            // Nerd Font: nf-dev-git_commit (U+E729)
            '\u{e729}'.to_string()
        } else {
            "●".to_string()
        }
    }

    #[allow(dead_code)]
    pub fn repo() -> String {
        if use_nerd_fonts() {
            // Nerd Font: nf-fa-github (U+F09B)
            '\u{f09b}'.to_string()
        } else {
            "⚡".to_string()
        }
    }

    #[allow(dead_code)]
    pub fn modified() -> String {
        if use_nerd_fonts() {
            // Nerd Font: nf-dev-git_merge (U+E727)
            '\u{e727}'.to_string()
        } else {
            "✎".to_string()
        }
    }

    pub fn owner() -> String {
        if use_nerd_fonts() {
            // Nerd Font: nf-fa-user (U+F007)
            '\u{f007}'.to_string()
        } else {
            "👤".to_string()
        }
    }
}

/// Status icons
pub mod status {
    use super::use_nerd_fonts;

    pub fn success() -> String {
        if use_nerd_fonts() {
            // Nerd Font: nf-fa-check_circle (U+F058)
            '\u{f058}'.to_string()
        } else {
            "✓".to_string()
        }
    }

    #[allow(dead_code)]
    pub fn error() -> String {
        if use_nerd_fonts() {
            // Nerd Font: nf-fa-times_circle (U+F057)
            '\u{f057}'.to_string()
        } else {
            "✗".to_string()
        }
    }

    #[allow(dead_code)]
    pub fn warning() -> String {
        if use_nerd_fonts() {
            // Nerd Font: nf-fa-exclamation_triangle (U+F071)
            '\u{f071}'.to_string()
        } else {
            "⚠".to_string()
        }
    }

    pub fn waiting() -> String {
        if use_nerd_fonts() {
            // Nerd Font: nf-fa-clock_o (U+F017)
            '\u{f017}'.to_string()
        } else {
            "⏳".to_string()
        }
    }

    pub fn running() -> String {
        if use_nerd_fonts() {
            // Nerd Font: nf-fa-cog (U+F013)
            '\u{f013}'.to_string()
        } else {
            "⚙".to_string()
        }
    }

    pub fn info() -> String {
        if use_nerd_fonts() {
            // Nerd Font: nf-fa-clock_o (U+F017)
            '\u{f017}'.to_string()
        } else {
            "🕒".to_string()
        }
    }
}

/// File and folder icons
pub mod files {
    use super::use_nerd_fonts;

    pub fn folder() -> String {
        if use_nerd_fonts() {
            // Nerd Font: nf-fa-folder (U+F07B)
            '\u{f07b}'.to_string()
        } else {
            "📁".to_string()
        }
    }

    #[allow(dead_code)]
    pub fn file() -> String {
        if use_nerd_fonts() {
            // Nerd Font: nf-fa-file (U+F016)
            '\u{f016}'.to_string()
        } else {
            "📄".to_string()
        }
    }

    #[allow(dead_code)]
    pub fn script() -> String {
        if use_nerd_fonts() {
            // Nerd Font: nf-oct-file_code (U+F010A)
            '\u{f010a}'.to_string()
        } else {
            "📜".to_string()
        }
    }
}
