//! Disk persistence, separate from GTK. Replace saved Projects atomically.
use crate::state::CrawlProject;
use std::{
    io,
    path::{Path, PathBuf},
};

pub const PROJECT_EXTENSION: &str = "tadpole";
pub const LEGACY_PROJECT_EXTENSION: &str = "seocrawl";
pub const DEFAULT_PROJECT_FILENAME: &str = "project.tadpole";
pub const PROJECT_MIME_TYPE: &str = "application/x-tadpole-project";
pub const SAVE_FILTER_PATTERNS: &[&str] = &["*.tadpole"];
pub const OPEN_FILTER_PATTERNS: &[&str] = &["*.tadpole", "*.seocrawl"];

pub fn ensure_project_save_path(path: &Path) -> PathBuf {
    if path.extension().is_none() {
        path.with_extension(PROJECT_EXTENSION)
    } else {
        path.to_path_buf()
    }
}

pub fn is_supported_project_path(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| {
            ext.eq_ignore_ascii_case(PROJECT_EXTENSION)
                || ext.eq_ignore_ascii_case(LEGACY_PROJECT_EXTENSION)
        })
}

pub fn resolve_open_project_path<I, P>(paths: I) -> Option<PathBuf>
where
    I: IntoIterator<Item = P>,
    P: AsRef<Path>,
{
    paths
        .into_iter()
        .map(|p| p.as_ref().to_path_buf())
        .find(|path| is_supported_project_path(path))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpenGuard {
    Proceed,
    ConfirmReplace { existing_urls: usize },
    BlockedByActiveCrawl,
}

pub fn open_guard(is_crawling: bool, is_busy: bool, existing_urls: usize) -> OpenGuard {
    if is_crawling || is_busy {
        OpenGuard::BlockedByActiveCrawl
    } else if existing_urls > 0 {
        OpenGuard::ConfirmReplace { existing_urls }
    } else {
        OpenGuard::Proceed
    }
}

pub fn save(
    path: &Path,
    project: &CrawlProject,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let directory = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let mut temporary = tempfile::NamedTempFile::new_in(directory)?;
    {
        let mut writer = io::BufWriter::new(temporary.as_file_mut());
        serde_json::to_writer(&mut writer, project)?;
        use io::Write;
        writer.flush()?;
    }
    temporary.as_file().sync_all()?;
    temporary.persist(path)?;
    Ok(())
}

pub fn load(path: &Path) -> Result<CrawlProject, Box<dyn std::error::Error + Send + Sync>> {
    let file = std::fs::File::open(path)?;
    let reader = io::BufReader::new(file);
    let project: CrawlProject = serde_json::from_reader(reader)?;
    project
        .config
        .validate()
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    Ok(project)
}

/// Use the complete URL hash as a suffix so identical slugs never overwrite each other.
pub fn markdown_filename(url: &str) -> String {
    // Stable FNV-1a; filenames must not change with Rust's DefaultHasher implementation.
    let hash = url.bytes().fold(0xcbf29ce484222325u64, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(0x100000001b3)
    });
    let slug: String = crate::crawler::get_slug_filename(url)
        .chars()
        .take(48)
        .collect();
    format!("{slug}-{hash:016x}.md")
}
