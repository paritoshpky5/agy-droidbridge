use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

const LEGACY: [(&str, &str); 11] = [
    ("gemini-3.5-flash-low", "Gemini 3.5 Flash (Low)"),
    ("gemini-3.5-flash-high", "Gemini 3.5 Flash (High)"),
    ("gpt-oss-120b-medium", "GPT-OSS 120B (Medium)"),
    ("claude-4.5-sonnet", "Claude 4.5 Sonnet"),
    ("claude-4.5-sonnet-thinking", "Claude 4.5 Sonnet (Thinking)"),
    ("claude-4.5-opus", "Claude 4.5 Opus"),
    ("claude-4.5-opus-thinking", "Claude 4.5 Opus (Thinking)"),
    ("claude-4.6-sonnet", "Claude 4.6 Sonnet"),
    ("claude-4.6-sonnet-thinking", "Claude 4.6 Sonnet (Thinking)"),
    ("claude-4.6-opus", "Claude 4.6 Opus"),
    ("claude-4.6-opus-thinking", "Claude 4.6 Opus (Thinking)"),
];

pub struct Model {
    pub id: String,
    pub label: String,
    pub owner: &'static str,
}

impl Model {
    fn new(id: &str, label: &str) -> Self {
        let owner = if id.starts_with("gemini") {
            "google"
        } else if id.starts_with("claude") {
            "anthropic"
        } else if id.starts_with("gpt-oss") {
            "openai"
        } else {
            "antigravity"
        };
        Self {
            id: id.into(),
            label: label.into(),
            owner,
        }
    }
}

pub struct Catalog(pub Vec<Model>);

impl Catalog {
    pub fn load(file: Option<&Path>) -> io::Result<Self> {
        match file {
            Some(file) => {
                let mut text = String::new();
                File::open(file)?.take(65_537).read_to_string(&mut text)?;
                Self::from_listing(&text)
            }
            None => Ok(Self(LEGACY.map(|(id, label)| Model::new(id, label)).into())),
        }
    }

    fn from_listing(text: &str) -> io::Result<Self> {
        let invalid = || io::Error::new(io::ErrorKind::InvalidData, "invalid model list");
        if text.len() > 65_536 {
            return Err(invalid());
        }
        let mut models: Vec<Model> = Vec::new();
        for row in text.lines().map(str::trim).filter(|row| !row.is_empty()) {
            let (id, label) = row.split_once(char::is_whitespace).unwrap_or((row, row));
            let label = label.trim();
            if id.len() > 128
                || label.is_empty()
                || label.len() > 256
                || !id.starts_with(|c: char| c.is_ascii_alphanumeric())
                || !id
                    .bytes()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || b".-_".contains(&c))
                || label.chars().any(char::is_control)
                || models
                    .iter()
                    .any(|model| model.id == id || model.label.eq_ignore_ascii_case(label))
            {
                return Err(invalid());
            }
            models.push(Model::new(id, label));
        }
        if models.is_empty() || models.len() > 256 {
            return Err(invalid());
        }
        Ok(Self(models))
    }

    pub fn resolve(&self, input: &str) -> Option<&Model> {
        let input = input.trim();
        let alias = match input.to_ascii_lowercase().as_str() {
            "gemini-3.5-flash" => "gemini-3.5-flash-low",
            "gpt-oss-120b" => "gpt-oss-120b-medium",
            _ => input,
        };
        self.0.iter().find(|model| {
            model.label.eq_ignore_ascii_case(input)
                || model.id.eq_ignore_ascii_case(input)
                || model.id.eq_ignore_ascii_case(alias)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_legacy_model_ids_and_aliases() {
        let catalog = Catalog::load(None).unwrap();
        assert_eq!(catalog.0.len(), 11);
        assert_eq!(
            catalog.resolve(" GEMINI-3.5-FLASH ").unwrap().id,
            LEGACY[0].0
        );
        assert_eq!(
            catalog.resolve("claude 4.6 opus (thinking)").unwrap().id,
            LEGACY[10].0
        );
        assert!(catalog.resolve("../../anything").is_none());
    }

    #[test]
    fn validates_account_catalogs_for_use_as_arguments() {
        let catalog =
            Catalog::from_listing("gemini-3.8-flash-medium   Gemini 3.8 Flash (Medium)\n").unwrap();
        assert!(catalog.resolve("gemini-3.8-flash-medium").is_some());
        for listing in [
            "",
            "--flag fake",
            "../bad tainted",
            "id A\nid B",
            "a A\nb a",
        ] {
            assert!(Catalog::from_listing(listing).is_err(), "{listing:?}");
        }
    }
}
