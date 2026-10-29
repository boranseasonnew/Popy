use crate::{
    pipeline::{Options, benchmark, write_json},
    process::is_cancelled,
    tools::{Model, Tools, external_path, sha256},
};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{collections::BTreeSet, fs, path::Path};

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub schema_version: u32,
    pub cases: Vec<Case>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Case {
    pub id: String,
    pub input: std::path::PathBuf,
    pub model: Model,
    pub seconds: f64,
    pub rights: String,
    #[serde(default = "one")]
    pub repeats: u32,
    #[serde(default)]
    pub gpu: u32,
    #[serde(default = "tile_size")]
    pub tile: u32,
    #[serde(default)]
    pub auto_color: bool,
}

fn one() -> u32 {
    1
}
fn tile_size() -> u32 {
    64
}

impl Manifest {
    pub fn validate(&self) -> Result<()> {
        ensure!(self.schema_version == 1, "Desteklenmeyen suite şeması");
        ensure!(
            !self.cases.is_empty() && self.cases.len() <= 12,
            "Suite 1–12 vaka içermeli"
        );
        let mut ids = BTreeSet::new();
        let mut jobs = 0;
        for case in &self.cases {
            ensure!(
                !case.id.is_empty()
                    && case.id.len() <= 48
                    && case
                        .id
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_'),
                "Vaka kimliği yalnızca ASCII harf, rakam, - ve _ içermeli (1–48 karakter)"
            );
            ensure!(
                ids.insert(&case.id),
                "Tekrar eden vaka kimliği: {}",
                case.id
            );
            ensure!((1..=3).contains(&case.repeats), "Tekrar sayısı 1–3 olmalı");
            ensure!(
                case.seconds.is_finite() && (0.1..=10.0).contains(&case.seconds),
                "Vaka süresi 0.1–10 saniye olmalı"
            );
            ensure!(
                case.gpu <= 15 && (32..=256).contains(&case.tile),
                "GPU veya tile değeri geçersiz"
            );
            ensure!(
                !case.rights.trim().is_empty(),
                "Her vaka için kullanım hakkı bilgisi gerekiyor"
            );
            ensure!(!case.input.as_os_str().is_empty(), "Girdi yolu boş olamaz");
            jobs += case.repeats;
        }
        ensure!(jobs <= 24, "Suite en fazla 24 iş çalıştırabilir");
        Ok(())
    }
}

fn load(path: &Path) -> Result<Manifest> {
    ensure!(
        fs::metadata(path)?.len() <= 128 * 1024,
        "Manifest 128 KiB sınırını aşıyor"
    );
    let manifest: Manifest =
        serde_json::from_slice(&fs::read(path)?).context("Suite manifesti okunamadı")?;
    manifest.validate()?;
    Ok(manifest)
}

/// No tools, model loads or output-directory creation are required for this step.
pub fn preview(path: &Path) -> Result<Value> {
    let manifest = load(path)?;
    let base = path.parent().unwrap_or(Path::new("."));
    let inputs: Vec<_> = manifest
        .cases
        .iter()
        .map(|case| {
            json!({
                "id":case.id, "input":case.input, "available":base.join(&case.input).is_file(),
                "model":case.model, "native_scale":case.model.native_scale(), "output_scale":2,
                "repeats":case.repeats, "seconds_per_job":case.seconds, "auto_color":case.auto_color
            })
        })
        .collect();
    Ok(
        json!({"state":"manifest_valid", "jobs":manifest.cases.iter().map(|c| c.repeats).sum::<u32>(),
        "note":"No inference performed. Missing inputs must be generated before execution.","cases":inputs}),
    )
}

#[derive(Serialize)]
struct Outcome {
    case_id: String,
    repeat: u32,
    state: &'static str,
    report: Option<String>,
    error: Option<String>,
    ai_seconds: Option<f64>,
}

pub fn run(tools: &Tools, manifest_path: &Path, output: &Path) -> Result<std::path::PathBuf> {
    let manifest_path = external_path(manifest_path)?;
    let manifest = load(&manifest_path)?;
    let base = manifest_path
        .parent()
        .context("Manifest klasörü bulunamadı")?;
    for case in &manifest.cases {
        ensure!(
            base.join(&case.input).is_file(),
            "Girdi bulunamadı: {}. Önce fixtures komutunu çalıştırın.",
            case.input.display()
        );
    }
    fs::create_dir_all(
        output
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new(".")),
    )?;
    fs::create_dir(output).context("Suite çıktı klasörü zaten var; yeni bir klasör seçin")?;
    let output = external_path(output)?;
    let manifest_hash = sha256(&manifest_path)?;
    write_json(
        &output.join("manifest.json"),
        &serde_json::to_value(&manifest)?,
    )?;
    let mut results = Vec::new();
    persist(&output, &manifest, &manifest_hash, &results, "running")?;
    'cases: for case in &manifest.cases {
        for repeat in 1..=case.repeats {
            if is_cancelled() {
                break 'cases;
            }
            let job_id = format!("{}-{repeat:02}", case.id);
            eprintln!("[suite] {job_id}");
            let result = benchmark(
                tools,
                Options {
                    input: base.join(&case.input),
                    output: output.join(&job_id),
                    model: case.model,
                    seconds: case.seconds,
                    gpu: case.gpu,
                    tile: case.tile,
                    auto_color: case.auto_color,
                    source_rights: case.rights.clone(),
                },
            )
            .and_then(|directory| {
                let report: Value =
                    serde_json::from_slice(&fs::read(directory.join("report.json"))?)?;
                ensure!(report["state"] == "completed", "Alt iş tamamlanmadı");
                let seconds = report["measurement"]["ai_seconds"]
                    .as_f64()
                    .context("AI ölçümü eksik")?;
                ensure!(seconds.is_finite() && seconds > 0.0, "AI ölçümü geçersiz");
                Ok(seconds)
            });
            let outcome = match result {
                Ok(seconds) => Outcome {
                    case_id: case.id.clone(),
                    repeat,
                    state: "completed",
                    report: Some(format!("{job_id}/report.json")),
                    error: None,
                    ai_seconds: Some(seconds),
                },
                Err(error) => Outcome {
                    case_id: case.id.clone(),
                    repeat,
                    state: "failed",
                    report: None,
                    error: Some(format!("{error:#}")),
                    ai_seconds: None,
                },
            };
            results.push(outcome);
            persist(&output, &manifest, &manifest_hash, &results, "running")?;
        }
    }
    let state = if is_cancelled() {
        "cancelled"
    } else if results.iter().any(|r| r.state != "completed") {
        "completed_with_errors"
    } else {
        "completed"
    };
    persist(&output, &manifest, &manifest_hash, &results, state)?;
    ensure!(
        state == "completed",
        "Suite {state}; sonuçlar {}",
        output.join("summary.json").display()
    );
    Ok(output)
}

fn median(values: &mut [f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    values.sort_by(f64::total_cmp);
    let half = values.len() / 2;
    Some(if values.len() % 2 == 0 {
        (values[half - 1] + values[half]) / 2.0
    } else {
        values[half]
    })
}

fn persist(
    output: &Path,
    manifest: &Manifest,
    hash: &str,
    results: &[Outcome],
    state: &str,
) -> Result<()> {
    let mut stats = Vec::new();
    let mut csv = String::from(
        "case_id,model,auto_color,completed,planned,median_ai_seconds,min_ai_seconds,max_ai_seconds\n",
    );
    for case in &manifest.cases {
        let mut values: Vec<_> = results
            .iter()
            .filter(|r| r.case_id == case.id)
            .filter_map(|r| r.ai_seconds)
            .collect();
        let median = median(&mut values);
        let minimum = values.first().copied();
        let maximum = values.last().copied();
        stats.push(json!({"case_id":case.id,"model":case.model,"auto_color":case.auto_color,
            "planned":case.repeats,"completed":values.len(),"median_ai_seconds":median,"min_ai_seconds":minimum,"max_ai_seconds":maximum}));
        let number = |v: Option<f64>| v.map(|v| format!("{v:.6}")).unwrap_or_default();
        csv.push_str(&format!(
            "{},{},{},{},{},{},{},{}\n",
            case.id,
            case.model.name(),
            case.auto_color,
            values.len(),
            case.repeats,
            number(median),
            number(minimum),
            number(maximum)
        ));
    }
    write_json(
        &output.join("summary.json"),
        &json!({
            "schema_version":1,"state":state,"manifest_sha256":hash,"app_version":env!("CARGO_PKG_VERSION"),
            "planned_jobs":manifest.cases.iter().map(|c|c.repeats).sum::<u32>(),"finished_jobs":results.len(),
            "quality_gate_passed":false,"note":"Cold process/model starts and PNG I/O included. No perceptual quality score. Failed jobs are excluded from timing statistics.",
            "cases":stats,"results":results
        }),
    )?;
    fs::write(output.join("summary.csv"), csv)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn valid() -> Value {
        json!({"schema_version":1,"cases":[{"id":"anime-144p","input":"test.mp4","model":"anime","seconds":1,"rights":"generated"}]})
    }
    fn check(value: Value) -> bool {
        serde_json::from_value::<Manifest>(value).is_ok_and(|m| m.validate().is_ok())
    }
    #[test]
    fn rejects_typos_and_unknown_models() {
        let mut v = valid();
        v["cases"][0]["auto_colour"] = true.into();
        assert!(!check(v));
        let mut v = valid();
        v["cases"][0]["model"] = "something".into();
        assert!(!check(v));
    }
    #[test]
    fn rejects_output_path_traversal_and_duplicate_ids() {
        for id in ["../escape", "a/b", "a\\b", "", "a,b", "a\nline"] {
            let mut v = valid();
            v["cases"][0]["id"] = id.into();
            assert!(!check(v));
        }
        let mut v = valid();
        let duplicate = v["cases"][0].clone();
        v["cases"].as_array_mut().unwrap().push(duplicate);
        assert!(!check(v));
    }
    #[test]
    fn validates_defaults_and_bounded_resources() {
        let m: Manifest = serde_json::from_value(valid()).unwrap();
        m.validate().unwrap();
        assert_eq!(m.cases[0].tile, 64);
        assert_eq!(m.cases[0].repeats, 1);
        for (key, value) in [
            ("seconds", json!(-1)),
            ("repeats", json!(0)),
            ("repeats", json!(4)),
            ("gpu", json!(16)),
            ("tile", json!(16)),
            ("rights", json!(" ")),
        ] {
            let mut v = valid();
            v["cases"][0][key] = value;
            assert!(!check(v));
        }
    }
    #[test]
    fn median_is_not_biased_by_repeat_order() {
        assert_eq!(median(&mut []), None);
        assert_eq!(median(&mut [8.0, 2.0, 4.0]), Some(4.0));
        assert_eq!(median(&mut [8.0, 2.0, 4.0, 6.0]), Some(5.0));
    }
    #[test]
    fn failures_do_not_become_zero_second_measurements() {
        let m: Manifest = serde_json::from_value(valid()).unwrap();
        let root = std::env::current_dir()
            .unwrap()
            .join("target/test-logs")
            .join(format!(
                "suite-{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
        fs::create_dir_all(&root).unwrap();
        let results = vec![Outcome {
            case_id: "anime-144p".into(),
            repeat: 1,
            state: "failed",
            report: None,
            error: Some("test error".into()),
            ai_seconds: None,
        }];
        persist(&root, &m, "test-hash", &results, "completed_with_errors").unwrap();
        let report: Value =
            serde_json::from_slice(&fs::read(root.join("summary.json")).unwrap()).unwrap();
        assert!(report["cases"][0]["median_ai_seconds"].is_null());
        assert_eq!(report["quality_gate_passed"], false);
        assert_eq!(report["state"], "completed_with_errors");
    }
}
