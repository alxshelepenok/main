use std::{
    error::Error,
    fs,
    io::Write,
    path::{Path, PathBuf},
};

use zip::{write::SimpleFileOptions, CompressionMethod, ZipWriter};

const ARCHIVE_FILE: &str = "site.zip";

pub(crate) fn write_site_zip(files: &[PathBuf]) -> Result<(), Box<dyn Error>> {
    write_zip(
        files,
        Path::new(super::OUTPUT_PATH),
        &super::output_path(ARCHIVE_FILE),
    )
}

fn write_zip(files: &[PathBuf], base: &Path, out_file: &Path) -> Result<(), Box<dyn Error>> {
    let mut entries: Vec<(String, &PathBuf)> = Vec::with_capacity(files.len());
    for file in files {
        entries.push((archive_name(file, base)?, file));
    }
    entries.sort();
    entries.dedup();

    let out = fs::File::create(out_file)
        .map_err(|e| format!("failed to create {}: {e}", out_file.display()))?;
    let mut zip = ZipWriter::new(out);
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    for (name, file) in &entries {
        let bytes = fs::read(file)
            .map_err(|e| format!("failed to read {} for archiving: {e}", file.display()))?;
        zip.start_file(name.as_str(), options)
            .map_err(|e| format!("failed to add {name} to the archive: {e}"))?;
        zip.write_all(&bytes)
            .map_err(|e| format!("failed to write {name} to the archive: {e}"))?;
    }
    zip.finish()
        .map(|_| ())
        .map_err(|e| format!("failed to finalize the archive: {e}").into())
}

fn archive_name(file: &Path, base: &Path) -> Result<String, Box<dyn Error>> {
    let relative = file
        .strip_prefix(base)
        .map_err(|_| format!("{} is outside the output directory", file.display()))?;
    let parts: Vec<&str> = relative
        .components()
        .filter_map(|c| match c {
            std::path::Component::Normal(part) => part.to_str(),
            _ => None,
        })
        .collect();
    if parts.is_empty() {
        return Err(format!("{} has no archive name", file.display()).into());
    }
    Ok(parts.join("/"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    #[test]
    fn archive_name_uses_forward_slashes_relative_to_the_base() {
        assert_eq!(archive_name(Path::new("target/index.html"), Path::new("target")).unwrap(), "index.html");
        assert_eq!(
            archive_name(
                Path::new("target/blog/tags/ai/index.html"),
                Path::new("target")
            )
            .unwrap(),
            "blog/tags/ai/index.html"
        );
        assert!(archive_name(Path::new("src/style.css"), Path::new("target")).is_err());
        assert!(archive_name(Path::new("target"), Path::new("target")).is_err());
    }

    #[test]
    fn write_zip_packs_every_file_under_its_relative_name() {
        let dir = std::env::temp_dir().join("main-tests-archive");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("index.html"), b"home").unwrap();
        let nested = dir.join("blog").join("one");
        fs::create_dir_all(&nested).unwrap();
        fs::write(nested.join("index.html"), b"body").unwrap();

        let out = dir.join("site.zip");
        write_zip(
            &[dir.join("index.html"), nested.join("index.html")],
            &dir,
            &out,
        )
        .unwrap();

        let file = fs::File::open(&out).unwrap();
        let mut zip = zip::ZipArchive::new(file).unwrap();
        let mut names: Vec<String> = zip.file_names().map(str::to_owned).collect();
        names.sort();
        assert_eq!(names, ["blog/one/index.html", "index.html"]);

        let mut entry = zip.by_name("blog/one/index.html").unwrap();
        let mut body = String::new();
        entry.read_to_string(&mut body).unwrap();
        assert_eq!(body, "body");
    }
}
