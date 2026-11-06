//! Disk persistence, separate from GTK. Replace saved projects atomically.
use crate::state::CrawlProject;
use std::{io, path::Path};

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
