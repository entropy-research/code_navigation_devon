use crate::{intelligence::TreeSitterFile, symbol::SymbolLocations, text_range::TextRange};

#[derive(Debug, Clone)]
pub struct ContentDocument {
    pub content: String,
    pub lang: Option<String>,
    pub relative_path: String,
    pub line_end_indices: Vec<u32>,
    pub symbol_locations: SymbolLocations,
}

impl ContentDocument {
    pub fn hoverable_ranges(&self) -> Option<Vec<TextRange>> {
        TreeSitterFile::try_build(self.content.as_bytes(), self.lang.as_ref()?)
            .and_then(TreeSitterFile::hoverable_ranges)
            .ok()
    }
}

impl std::hash::Hash for ContentDocument {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.relative_path.hash(state);
        self.content.hash(state);
    }
}

impl PartialEq for ContentDocument {
    fn eq(&self, other: &Self) -> bool {
        self.relative_path == other.relative_path && self.content == other.content
    }
}
impl Eq for ContentDocument {}
