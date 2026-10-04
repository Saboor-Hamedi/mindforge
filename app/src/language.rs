//! Central language identity and file-extension detection for editor documents.

use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FileLanguage {
    Markdown,
    JavaScript,
    Jsx,
    TypeScript,
    Tsx,
    Python,
    Rust,
    Php,
    Html,
    Css,
    Json,
    Yaml,
    Sql,
    PlainText,
}

impl FileLanguage {
    pub const ALL: [Self; 14] = [
        Self::Markdown,
        Self::JavaScript,
        Self::TypeScript,
        Self::Jsx,
        Self::Tsx,
        Self::Python,
        Self::Rust,
        Self::Php,
        Self::Html,
        Self::Css,
        Self::Json,
        Self::Yaml,
        Self::Sql,
        Self::PlainText,
    ];

    /// Detects a language from a note title such as `app.jsx`. Returns `None`
    /// when the title has no recognised code extension so plain notes stay Markdown.
    pub fn from_title(title: &str) -> Option<Self> {
        let extension = Path::new(title.trim()).extension()?;
        match Self::from_path(Path::new(&format!("x.{}", extension.to_str()?))) {
            Self::PlainText => None,
            language => Some(language),
        }
    }

    pub fn from_path(path: &Path) -> Self {
        let Some(extension) = path.extension().and_then(|extension| extension.to_str()) else {
            return Self::PlainText;
        };
        match extension.to_ascii_lowercase().as_str() {
            "md" | "markdown" | "mdown" => Self::Markdown,
            "js" | "mjs" | "cjs" => Self::JavaScript,
            "jsx" => Self::Jsx,
            "ts" | "mts" | "cts" => Self::TypeScript,
            "tsx" => Self::Tsx,
            "py" | "pyw" => Self::Python,
            "rs" => Self::Rust,
            "php" | "phtml" => Self::Php,
            "html" | "htm" => Self::Html,
            "css" => Self::Css,
            "json" | "jsonc" => Self::Json,
            "yaml" | "yml" => Self::Yaml,
            "sql" => Self::Sql,
            _ => Self::PlainText,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Markdown => "Markdown",
            Self::JavaScript => "JavaScript",
            Self::Jsx => "JSX",
            Self::TypeScript => "TypeScript",
            Self::Tsx => "TSX",
            Self::Python => "Python",
            Self::Rust => "Rust",
            Self::Php => "PHP",
            Self::Html => "HTML",
            Self::Css => "CSS",
            Self::Json => "JSON",
            Self::Yaml => "YAML",
            Self::Sql => "SQL",
            Self::PlainText => "Plain Text",
        }
    }

    pub fn nvim_filetype(self) -> &'static str {
        match self {
            Self::Markdown => "markdown",
            Self::JavaScript => "javascript",
            Self::Jsx => "javascriptreact",
            Self::TypeScript => "typescript",
            Self::Tsx => "typescriptreact",
            Self::Python => "python",
            Self::Rust => "rust",
            Self::Php => "php",
            Self::Html => "html",
            Self::Css => "css",
            Self::Json => "json",
            Self::Yaml => "yaml",
            Self::Sql => "sql",
            Self::PlainText => "text",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LanguageSelection {
    AutoDetect,
    Language(FileLanguage),
}

#[derive(Debug, Clone, Default)]
pub struct LanguageSelectorState {
    pub open: bool,
    pub query: String,
    pub selected_index: usize,
    pub focus_search: bool,
}

#[cfg(test)]
mod tests {
    use super::FileLanguage;
    use std::path::Path;

    #[test]
    fn plain_titles_default_to_markdown() {
        assert_eq!(FileLanguage::from_title("Hello World"), None);
        assert_eq!(FileLanguage::from_title("meeting.notes"), None);
        assert_eq!(FileLanguage::from_title("hello.py"), Some(FileLanguage::Python));
    }
    #[test]
    fn detects_supported_extensions_case_insensitively() {
        let cases = [
            ("my_book.md", FileLanguage::Markdown),
            ("app.js", FileLanguage::JavaScript),
            ("view.jsx", FileLanguage::Jsx),
            ("main.ts", FileLanguage::TypeScript),
            ("App.tsx", FileLanguage::Tsx),
            ("main.py", FileLanguage::Python),
            ("main.rs", FileLanguage::Rust),
            ("index.php", FileLanguage::Php),
            ("index.htm", FileLanguage::Html),
            ("styles.css", FileLanguage::Css),
            ("package.json", FileLanguage::Json),
            ("config.YAML", FileLanguage::Yaml),
            ("query.sql", FileLanguage::Sql),
            ("unknown.xyz", FileLanguage::PlainText),
            (".env", FileLanguage::PlainText),
        ];
        for (path, expected) in cases {
            assert_eq!(FileLanguage::from_path(Path::new(path)), expected, "{path}");
        }
    }
}
