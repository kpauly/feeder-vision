use anyhow::{Context, Result, anyhow};
use image::{DynamicImage, RgbaImage, imageops::FilterType};
use ndarray::{Array4, CowArray};
use once_cell::sync::Lazy;
use ort::{
    GraphOptimizationLevel, SessionBuilder, environment::Environment, session::Session,
    tensor::OrtOwnedTensor, value::Value,
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
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
    /// Whether an animal/bird is present.
    pub present: bool,
    /// Optional classifier output.
    pub classification: Option<Classification>,
}

/// Options controlling how folder scanning behaves.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScanOptions {
    /// When true, scan subdirectories recursively.
    pub recursive: bool,
}

/// Scan a folder for images and produce basic `ImageInfo` entries.
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
    wtr.write_record(["file", "present", "species", "confidence"])?;

    for info in rows {
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

fn is_supported_image(path: &Path) -> bool {
    match path.extension().and_then(|s| s.to_str()) {
        Some(ext) => {
            let ext = ext.to_ascii_lowercase();
            matches!(ext.as_str(), "jpg" | "jpeg" | "png")
        }
        None => false,
    }
}

static ORT_ENV: Lazy<Arc<Environment>> = Lazy::new(|| {
    Environment::builder()
        .with_name("feeder-vision")
        .build()
        .expect("failed to initialize ONNX Runtime environment")
        .into_arc()
});

/// Configuration for the ONNX-based EfficientNet classifier.
#[derive(Debug, Clone)]
pub struct ClassifierConfig {
    pub model_path: PathBuf,
    pub labels_path: PathBuf,
    pub input_size: u32,
    pub presence_threshold: f32,
    pub mean: [f32; 3],
    pub std: [f32; 3],
}

impl Default for ClassifierConfig {
    fn default() -> Self {
        Self {
            model_path: PathBuf::from("models/efficientnet_b0.onnx"),
            labels_path: PathBuf::from("models/labels.txt"),
            input_size: 512,
            presence_threshold: 0.5,
            mean: [0.485, 0.456, 0.406],
            std: [0.229, 0.224, 0.225],
        }
    }
}

/// EfficientNet classifier backed by ONNX Runtime.
pub struct EfficientNetOrt {
    session: Session,
    labels: Vec<String>,
    input_size: u32,
    presence_threshold: f32,
    mean: [f32; 3],
    std: [f32; 3],
}

impl EfficientNetOrt {
    pub fn new(cfg: &ClassifierConfig) -> Result<Self> {
        if !cfg.model_path.exists() {
            anyhow::bail!(
                "Modelbestand ontbreekt: {}",
                cfg.model_path.to_string_lossy()
            );
        }
        if !cfg.labels_path.exists() {
            anyhow::bail!(
                "Labels-bestand ontbreekt: {}",
                cfg.labels_path.to_string_lossy()
            );
        }
        let env = ORT_ENV.clone();
        let session = SessionBuilder::new(&env)?
            .with_optimization_level(GraphOptimizationLevel::Level1)?
            .with_model_from_file(&cfg.model_path)?;

        let labels_raw = fs::read_to_string(&cfg.labels_path).context("labels niet te lezen")?;
        let mut labels: Vec<String> = labels_raw
            .lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty())
            .map(|l| l.to_string())
            .collect();
        if labels.is_empty() {
            anyhow::bail!("labels-bestand bevat geen labels");
        }
        // ensure stable ordering
        labels.dedup();

        Ok(Self {
            session,
            labels,
            input_size: cfg.input_size,
            presence_threshold: cfg.presence_threshold,
            mean: cfg.mean,
            std: cfg.std,
        })
    }

    pub fn classify_with_progress<F>(&self, rows: &mut [ImageInfo], mut progress: F) -> Result<()>
    where
        F: FnMut(usize, usize),
    {
        let total = rows.len();
        if total == 0 {
            return Ok(());
        }
        for (idx, info) in rows.iter_mut().enumerate() {
            match self.classify_single(&info.file) {
                Ok(result) => {
                    info.present = result.present;
                    info.classification = result.classification;
                }
                Err(err) => {
                    tracing::warn!("Classifier fout voor {}: {err}", info.file.display());
                    info.present = false;
                    info.classification = None;
                }
            }
            progress(idx + 1, total);
        }
        Ok(())
    }

    fn classify_single(&self, path: &Path) -> Result<ClassificationResult> {
        let tensor = self.prepare_input(path)?;
        let input_array = tensor.into_dyn();
        let cow = CowArray::from(input_array.view());
        let input = Value::from_array(self.session.allocator(), &cow)
            .map_err(|e| anyhow!("kon inputtensor niet bouwen: {e}"))?;
        let outputs: Vec<Value> = self.session.run(vec![input])?;
        if outputs.is_empty() {
            anyhow::bail!("model gaf geen output");
        }
        let logits: OrtOwnedTensor<f32, _> = outputs[0].try_extract()?;
        let view = logits.view();
        let scores: Vec<f32> = view.iter().cloned().collect();
        if scores.is_empty() {
            anyhow::bail!("lege logits");
        }
        let probs = softmax(&scores);
        let (best_idx, &best_prob) = probs
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap();
        let label = self
            .labels
            .get(best_idx)
            .cloned()
            .unwrap_or_else(|| format!("class_{best_idx}"));
        let present = best_prob >= self.presence_threshold;
        let classification = if present {
            Some(Classification {
                decision: Decision::Label(label),
                confidence: best_prob,
            })
        } else {
            Some(Classification {
                decision: Decision::Unknown,
                confidence: best_prob,
            })
        };
        Ok(ClassificationResult {
            present,
            classification,
        })
    }

    fn prepare_input(&self, path: &Path) -> Result<Array4<f32>> {
        let img = image::open(path)
            .with_context(|| format!("kan afbeelding niet openen: {}", path.display()))?;
        let resized = resize_to_square(img, self.input_size);
        let mut array =
            Array4::<f32>::zeros((1, 3, self.input_size as usize, self.input_size as usize));
        for (x, y, pixel) in resized.enumerate_pixels() {
            let [r, g, b, _] = pixel.0;
            let coords = (y as usize, x as usize);
            array[[0, 0, coords.0, coords.1]] = normalize_channel(r, self.mean[0], self.std[0]);
            array[[0, 1, coords.0, coords.1]] = normalize_channel(g, self.mean[1], self.std[1]);
            array[[0, 2, coords.0, coords.1]] = normalize_channel(b, self.mean[2], self.std[2]);
        }
        Ok(array)
    }
}

fn resize_to_square(img: DynamicImage, size: u32) -> RgbaImage {
    img.resize_exact(size, size, FilterType::Triangle)
        .to_rgba8()
}

fn normalize_channel(value: u8, mean: f32, std: f32) -> f32 {
    let v = value as f32 / 255.0;
    (v - mean) / std
}

fn softmax(logits: &[f32]) -> Vec<f32> {
    let max = logits.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let exps: Vec<f32> = logits.iter().map(|&x| (x - max).exp()).collect();
    let sum: f32 = exps.iter().sum();
    if sum == 0.0 {
        return vec![0.0; logits.len()];
    }
    exps.into_iter().map(|x| x / sum).collect()
}

struct ClassificationResult {
    present: bool,
    classification: Option<Classification>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use tempfile::tempdir;

    #[test]
    fn export_csv_writes_expected_headers_and_rows() -> Result<()> {
        let dir = tempdir()?;
        let path = dir.path().join("out.csv");
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
        let headers = rdr.headers()?.clone();
        assert_eq!(
            headers.iter().collect::<Vec<_>>(),
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
        File::create(dir.path().join("a.JPG"))?;
        File::create(dir.path().join("b.jpeg"))?;
        File::create(dir.path().join("c.png"))?;
        File::create(dir.path().join("not-image.txt"))?;
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
}
