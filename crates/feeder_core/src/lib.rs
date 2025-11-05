use anyhow::Result;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Classification decision for an image/crop.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Decision {
    /// Abstain from labeling; treat as unknown class.
    Unknown,
    /// Labeled with a species name.
    Label(String),
}

/// Classification result with decision and confidence.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Classification {
    pub decision: Decision,
    /// Model similarity/confidence in [0,1].
    pub confidence: f32,
}

/// Core image information gathered by the pipeline.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImageInfo {
    pub file: PathBuf,
    /// Stage A result: whether a bird is present.
    pub present: bool,
    /// Optional classification (when present is true and a decision was made).
    pub classification: Option<Classification>,
}

/// Options controlling how folder scanning behaves.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScanOptions {
    /// When true, scan subdirectories recursively.
    pub recursive: bool,
}

/// Scan a folder for images and produce basic `ImageInfo` entries.
///
/// C1: lists jpg/jpeg/png files (non-recursive by default).
pub fn scan_folder(path: impl AsRef<Path>) -> Result<Vec<ImageInfo>> {
    scan_folder_with(path, ScanOptions::default())
}

/// Scan a folder with options.
pub fn scan_folder_with(path: impl AsRef<Path>, opts: ScanOptions) -> Result<Vec<ImageInfo>> {
    let root = path.as_ref();
    if !root.exists() {
        anyhow::bail!("Path does not exist: {}", root.display());
    }
    if !root.is_dir() {
        anyhow::bail!("Path is not a directory: {}", root.display());
    }

    let mut infos: Vec<ImageInfo> = Vec::new();

    let walker = if opts.recursive {
        WalkDir::new(root).into_iter()
    } else {
        WalkDir::new(root).max_depth(1).into_iter()
    };

    for entry in walker {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                // Skip unreadable entries, but keep scanning others
                tracing::warn!("walkdir error: {}", e);
                continue;
            }
        };
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if is_supported_image(path) {
            infos.push(ImageInfo {
                file: path.to_path_buf(),
                present: false,
                classification: None,
            });
        }
    }

    Ok(infos)
}

/// Export the provided rows to CSV with headers:
/// file,present,species,confidence
pub fn export_csv(rows: &[ImageInfo], path: impl AsRef<Path>) -> Result<()> {
    let mut wtr = csv::Writer::from_path(path)?;
    // Explicit header for clarity and stability
    wtr.write_record(["file", "present", "species", "confidence"])?;

    for info in rows {
        // Compute species/confidence fields
        let (species, confidence): (Option<String>, Option<f32>) = if info.present {
            match &info.classification {
                Some(Classification {
                    decision,
                    confidence,
                }) => {
                    let s = match decision {
                        Decision::Unknown => Some("Unknown".to_string()),
                        Decision::Label(name) => Some(name.clone()),
                    };
                    (s, Some(*confidence))
                }
                None => (None, None),
            }
        } else {
            (None, None)
        };

        // Convert Option to empty field when None
        let species_field = species.unwrap_or_default();
        let confidence_field = confidence
            .map(|c| format!("{c}"))
            .unwrap_or_else(String::new);

        wtr.write_record([
            info.file.to_string_lossy().as_ref(),
            if info.present { "true" } else { "false" },
            species_field.as_str(),
            confidence_field.as_str(),
        ])?;
    }

    wtr.flush()?;
    Ok(())
}

/// Presence detector interface allowing different backends.
pub trait PresenceDetector: Send + Sync {
    fn detect_present(&self, path: &Path) -> Result<bool>;
}

/// A lightweight CPU heuristic detector using intensity variance.
/// Fast and dependency-light; serves as MVP stub.
#[derive(Debug, Clone, Copy)]
pub struct HeuristicDetector {
    /// Threshold on grayscale standard deviation [0-255].
    pub stddev_threshold: f32,
    /// Optional downscale size for speed; 0 means no downscale.
    pub sample_size: u32,
}

impl Default for HeuristicDetector {
    fn default() -> Self {
        Self {
            stddev_threshold: 10.0,
            sample_size: 64,
        }
    }
}

impl PresenceDetector for HeuristicDetector {
    fn detect_present(&self, path: &Path) -> Result<bool> {
        let img = image::open(path)?;
        let mut gray = img.to_luma8();
        if self.sample_size > 0 {
            gray = image::imageops::thumbnail(&gray, self.sample_size, self.sample_size);
        }
        let mut sum = 0f64;
        let mut sum2 = 0f64;
        let n = (gray.width() as usize) * (gray.height() as usize);
        for p in gray.pixels() {
            let v = p[0] as f64;
            sum += v;
            sum2 += v * v;
        }
        let mean = sum / n as f64;
        let var = (sum2 / n as f64) - (mean * mean);
        let stddev = var.max(0.0).sqrt() as f32;
        Ok(stddev >= self.stddev_threshold)
    }
}

/// Apply presence detection to the provided image infos in-place.
pub fn apply_presence<D: PresenceDetector + ?Sized>(rows: &mut [ImageInfo], detector: &D) {
    rows.par_iter_mut()
        .for_each(|info| match detector.detect_present(&info.file) {
            Ok(p) => info.present = p,
            Err(e) => {
                tracing::warn!(
                    "presence detection failed for {}: {}",
                    info.file.display(),
                    e
                );
                info.present = false;
            }
        });
}

/// Convenience: scan a folder and immediately run detection.
pub fn scan_folder_detect(
    path: impl AsRef<Path>,
    opts: ScanOptions,
    detector: &dyn PresenceDetector,
) -> Result<Vec<ImageInfo>> {
    let mut rows = scan_folder_with(path, opts)?;
    apply_presence(&mut rows, detector);
    Ok(rows)
}

fn is_supported_image(path: &Path) -> bool {
    match path.extension().and_then(|s| s.to_str()) {
        Some(ext) => {
            let ext = ext.to_ascii_lowercase();
            matches!(ext.as_str(), "jpg" | "jpeg" | "png")
        }
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use tempfile::NamedTempFile;
    use tempfile::tempdir;

    #[test]
    fn export_csv_writes_expected_headers_and_rows() -> Result<()> {
        let tmp = NamedTempFile::new()?;
        let path = tmp.path().to_path_buf();

        let rows = vec![
            ImageInfo {
                file: PathBuf::from("a.jpg"),
                present: false,
                classification: None,
            },
            ImageInfo {
                file: PathBuf::from("b.jpg"),
                present: true,
                classification: Some(Classification {
                    decision: Decision::Unknown,
                    confidence: 0.42,
                }),
            },
            ImageInfo {
                file: PathBuf::from("c.jpg"),
                present: true,
                classification: Some(Classification {
                    decision: Decision::Label("Sparrow".into()),
                    confidence: 0.91,
                }),
            },
        ];

        export_csv(&rows, &path)?;

        let mut rdr = csv::Reader::from_path(&path)?;
        // header check
        let headers = rdr.headers()?.clone();
        let headers_vec: Vec<&str> = headers.iter().collect();
        assert_eq!(
            headers_vec,
            vec!["file", "present", "species", "confidence"]
        );

        let mut recs = rdr.records();
        let r1 = recs.next().unwrap()?;
        assert_eq!(&r1[0], "a.jpg");
        assert_eq!(&r1[1], "false");
        assert_eq!(&r1[2], "");
        assert_eq!(&r1[3], "");

        let r2 = recs.next().unwrap()?;
        assert_eq!(&r2[0], "b.jpg");
        assert_eq!(&r2[1], "true");
        assert_eq!(&r2[2], "Unknown");
        assert_eq!(&r2[3], "0.42");

        let r3 = recs.next().unwrap()?;
        assert_eq!(&r3[0], "c.jpg");
        assert_eq!(&r3[1], "true");
        assert_eq!(&r3[2], "Sparrow");
        assert_eq!(&r3[3], "0.91");

        assert!(recs.next().is_none());
        Ok(())
    }

    #[test]
    fn scan_folder_empty_returns_empty() -> Result<()> {
        let dir = tempdir()?;
        let rows = scan_folder(dir.path())?;
        assert!(rows.is_empty());
        Ok(())
    }

    #[test]
    fn scan_folder_lists_only_images_non_recursive() -> Result<()> {
        let dir = tempdir()?;
        // Files in root
        File::create(dir.path().join("a.JPG"))?;
        File::create(dir.path().join("b.jpeg"))?;
        File::create(dir.path().join("c.png"))?;
        File::create(dir.path().join("not-image.txt"))?;
        // Nested image should be ignored in non-recursive mode
        let nested = dir.path().join("nested");
        fs::create_dir(&nested)?;
        File::create(nested.join("d.jpg"))?;

        let rows = scan_folder_with(dir.path(), ScanOptions { recursive: false })?;
        let mut files: Vec<String> = rows
            .into_iter()
            .map(|i| i.file.file_name().unwrap().to_string_lossy().to_string())
            .collect();
        files.sort();
        assert_eq!(files, vec!["a.JPG", "b.jpeg", "c.png"]);
        Ok(())
    }

    #[test]
    fn scan_folder_lists_images_recursive_when_enabled() -> Result<()> {
        let dir = tempdir()?;
        File::create(dir.path().join("a.jpg"))?;
        let nested = dir.path().join("nested");
        fs::create_dir(&nested)?;
        File::create(nested.join("b.PNG"))?;

        let rows = scan_folder_with(dir.path(), ScanOptions { recursive: true })?;
        let mut files: Vec<String> = rows
            .into_iter()
            .map(|i| i.file.file_name().unwrap().to_string_lossy().to_string())
            .collect();
        files.sort();
        assert_eq!(files, vec!["a.jpg", "b.PNG"]);
        Ok(())
    }

    #[test]
    fn heuristic_detector_blank_vs_shape() -> Result<()> {
        let dir = tempdir()?;
        // Blank white image
        let blank_path = dir.path().join("blank.png");
        let blank = image::GrayImage::from_pixel(64, 64, image::Luma([255]));
        blank.save(&blank_path)?;

        // Image with a black rectangle
        let rect_path = dir.path().join("rect.png");
        let mut rect = image::GrayImage::from_pixel(64, 64, image::Luma([255]));
        for y in 16..48 {
            for x in 16..48 {
                rect.put_pixel(x, y, image::Luma([0]));
            }
        }
        rect.save(&rect_path)?;

        let det = HeuristicDetector::default();
        let p_blank = det.detect_present(&blank_path)?;
        let p_rect = det.detect_present(&rect_path)?;
        assert!(!p_blank);
        assert!(p_rect);
        Ok(())
    }
}
