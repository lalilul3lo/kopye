// TEST:
pub fn normalize_path(source: &String) -> std::path::PathBuf {
    let input = std::path::PathBuf::from(source);

    let mut new_path = std::path::PathBuf::new();

    for component in input.components() {
        match component {
            // Skip the current-dir marker "."
            std::path::Component::CurDir => {}

            // For "..", pop the last component if possible
            std::path::Component::ParentDir => {
                new_path.pop();
            }

            // For normal components, push them
            other => new_path.push(other.as_os_str()),
        }
    }

    new_path
}
