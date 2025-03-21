/// Represents a virtual file or directory entry to be created in memory before writing to disk.
///
/// This struct can be used to stage content for a file system operation such as rendering
/// templates into a virtual environment.
#[derive(Debug, Clone)]
pub struct VirtualEntry {
    /// The target path where the file or directory should be written. If `None`,
    /// the entry may be skipped or dynamically resolved.
    pub destination: Option<std::path::PathBuf>,
    /// Optional contents to be written if the entry represents a file.
    pub content: Option<String>,
    /// Indicates whether this entry is a file (`true`) or a directory (`false`).
    pub is_file: bool,
}
/// Represents a virtual file system composed of multiple [`VirtualEntry`] values.
///
/// This structure can be used to queue up a collection of file or directory creations
/// before committing them to disk.
#[derive(Debug, Clone)]
pub struct VirtualFS {
    pub entries: Vec<VirtualEntry>,
}
impl VirtualFS {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }
}
