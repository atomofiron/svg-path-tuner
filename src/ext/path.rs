use std::path::Path;

/// Whether the path names an .xml file, case insensitively.
pub fn is_xml(path: &Path) -> bool {
    path.extension().is_some_and(|extension| extension.eq_ignore_ascii_case("xml"))
}
