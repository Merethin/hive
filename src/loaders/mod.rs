mod categories;
mod metadata;
mod parameters;
mod templates;

use std::path::Path;

pub fn extension(path: &Path) -> Option<&str> {
    path.extension().and_then(|e| e.to_str())
}

pub fn file_prefix(path: &Path) -> Option<String> {
    path.file_prefix().map(|v| v.to_string_lossy().to_string())
}

pub use metadata::{TemplateMetadata, read_template_metadata};
pub use parameters::load_parameters;
pub use templates::{load_main_templates, load_supporting_templates};