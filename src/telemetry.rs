use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChapterStat {
    pub name: String,
    pub word_count: usize,
    pub plan_duration_secs: u64,
    pub prose_duration_secs: u64,
    pub review_duration_secs: u64,
    pub is_revised: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TelemetryData {
    pub total_words: usize,
    pub chapter_stats: Vec<ChapterStat>,
    pub active_flags: usize,
    pub target_words: usize,
    pub target_chapters: usize,
    pub last_updated: String,
}

pub struct TelemetryEngine {
    project_dir: PathBuf,
}

impl TelemetryEngine {
    pub fn new(project_dir: impl Into<PathBuf>) -> Self {
        Self {
            project_dir: project_dir.into(),
        }
    }

    /// Scans the project and recalculates all telemetry data.
    pub fn calculate(&self, target_words: usize, target_chapters: usize) -> TelemetryData {
        let mut total_words = 0;
        let mut chapter_stats = Vec::new();
        let mut active_flags = 0;

        // 1. Scan Prose for word counts and base chapters
        let prose_dir = self.project_dir.join("prose");
        if prose_dir.exists() && prose_dir.is_dir() {
            if let Ok(entries) = fs::read_dir(&prose_dir) {
                let mut files: Vec<_> = entries.filter_map(Result::ok).collect();
                files.sort_by_key(|e| e.file_name());
                
                for entry in files {
                    let path = entry.path();
                    if path.is_file() && path.extension().unwrap_or_default() == "md" {
                        if let Ok(content) = fs::read_to_string(&path) {
                            let count = content.split_whitespace().count();
                            total_words += count;
                            
                            let name = entry.file_name().to_string_lossy().into_owned();
                            // Parse chapter identifier (e.g. "ch01") to find matching plan/review
                            let ch_prefix = name.split('_').next().unwrap_or(&name);
                            
                            let prose_duration_secs = Self::get_file_duration(&path);
                            
                            let plan_path = self.project_dir.join("plan").join(format!("{}_plan.md", ch_prefix));
                            let plan_duration_secs = Self::get_file_duration(&plan_path);
                            
                            let review_path = self.project_dir.join("reviews").join(format!("{}_review.md", ch_prefix));
                            let review_duration_secs = Self::get_file_duration(&review_path);
                            
                            // Check if revised (heuristic: modified > created by a large margin, or just if the string is present)
                            let is_revised = content.contains("Phase 6 Revisions"); // Placeholder heuristic
                            
                            chapter_stats.push(ChapterStat {
                                name: ch_prefix.to_string(),
                                word_count: count,
                                plan_duration_secs,
                                prose_duration_secs,
                                review_duration_secs,
                                is_revised,
                            });
                        }
                    }
                }
            }
        }

        // 2. Scan Reviews for active continuity flags
        let reviews_dir = self.project_dir.join("reviews");
        if reviews_dir.exists() && reviews_dir.is_dir() {
            if let Ok(entries) = fs::read_dir(&reviews_dir) {
                for entry in entries.filter_map(Result::ok) {
                    let path = entry.path();
                    if path.is_file() && path.extension().unwrap_or_default() == "md" {
                        if let Ok(content) = fs::read_to_string(&path) {
                            // Simple heuristic: count the number of lines starting with "- [ ]" as active flags/tasks
                            let open_tasks = content.lines().filter(|l| l.trim_start().starts_with("- [ ]")).count();
                            active_flags += open_tasks;
                        }
                    }
                }
            }
        }

        let now = chrono::Utc::now().to_rfc3339();

        TelemetryData {
            total_words,
            chapter_stats,
            active_flags,
            target_words,
            target_chapters,
            last_updated: now,
        }
    }

    /// Recalculates and saves to .architext/stats.json
    pub fn update_and_save(&self, target_words: usize, target_chapters: usize) -> Result<TelemetryData, anyhow::Error> {
        let data = self.calculate(target_words, target_chapters);
        
        let stats_dir = self.project_dir.join(".architext");
        if !stats_dir.exists() {
            fs::create_dir_all(&stats_dir)?;
        }
        
        let stats_file = stats_dir.join("stats.json");
        let json = serde_json::to_string_pretty(&data)?;
        fs::write(stats_file, json)?;
        
        Ok(data)
    }

    /// Load the last saved telemetry without recalculating
    pub fn load_latest(&self) -> Option<TelemetryData> {
        let stats_file = self.project_dir.join(".architext").join("stats.json");
        if let Ok(content) = fs::read_to_string(stats_file) {
            serde_json::from_str(&content).ok()
        } else {
            None
        }
    }

    /// Helper to extract duration from file metadata (modified - created)
    fn get_file_duration(path: &Path) -> u64 {
        if let Ok(meta) = fs::metadata(path) {
            if let (Ok(created), Ok(modified)) = (meta.created(), meta.modified()) {
                if let Ok(duration) = modified.duration_since(created) {
                    return duration.as_secs();
                }
            }
        }
        0
    }
}
