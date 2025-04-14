use lingua::Language;

// Enum for target languages
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TargetLanguage {
    Portuguese,
    English,
    Ukrainian,
    Russian,
}

impl TargetLanguage {
    pub fn as_str(&self) -> &'static str {
        match self {
            TargetLanguage::Portuguese => "European Portuguese",
            TargetLanguage::English => "English",
            TargetLanguage::Ukrainian => "Ukrainian",
            TargetLanguage::Russian => "Russian",
        }
    }
    pub fn code(&self) -> &'static str {
        match self {
            TargetLanguage::Portuguese => "PT",
            TargetLanguage::English => "EN",
            TargetLanguage::Ukrainian => "UK",
            TargetLanguage::Russian => "RU",
        }
    }
    // Helper to parse from code
    pub fn from_code(code: &str) -> Option<Self> {
        match code {
            "PT" => Some(TargetLanguage::Portuguese),
            "EN" => Some(TargetLanguage::English),
            "UK" => Some(TargetLanguage::Ukrainian),
            "RU" => Some(TargetLanguage::Russian),
            _ => None,
        }
    }
    // Helper to convert from lingua::Language
    pub fn from_lingua(lang: Language) -> Option<Self> {
        match lang {
            Language::Portuguese => Some(TargetLanguage::Portuguese),
            Language::English => Some(TargetLanguage::English),
            Language::Ukrainian => Some(TargetLanguage::Ukrainian),
            Language::Russian => Some(TargetLanguage::Russian),
            _ => None, // Handle other languages if needed, or ignore
        }
    }
    // Helper to convert to lingua::Language
    #[allow(dead_code)] // May not be used after refactor, but keep for potential future use
    pub fn to_lingua(&self) -> Option<Language> {
        match self {
            TargetLanguage::Portuguese => Some(Language::Portuguese),
            TargetLanguage::English => Some(Language::English),
            TargetLanguage::Ukrainian => Some(Language::Ukrainian),
            TargetLanguage::Russian => Some(Language::Russian),
        }
    }
}
