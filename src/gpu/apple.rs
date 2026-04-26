use std::process::Command;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::error::Result;
use super::backend::{GPUBackend, GPUProcess};

const CACHE_TTL: Duration = Duration::from_millis(500);

struct PerfStats {
    utilization: Option<f32>,
    vram_used: Option<u64>,
    vram_free: Option<u64>,
    updated_at: Instant,
}

pub struct AppleBackend {
    name: String,
    cache: Mutex<Option<PerfStats>>,
}

impl AppleBackend {
    pub fn detect_all() -> Result<Vec<Self>> {
        let name = detect_gpu_name().unwrap_or_else(|| "Apple GPU".to_string());
        Ok(vec![AppleBackend {
            name,
            cache: Mutex::new(None),
        }])
    }

    fn perf_stats(&self) -> (Option<f32>, Option<u64>, Option<u64>) {
        let mut cache = self.cache.lock().unwrap_or_else(|e| e.into_inner());

        if let Some(ref stats) = *cache {
            if stats.updated_at.elapsed() < CACHE_TTL {
                return (stats.utilization, stats.vram_used, stats.vram_free);
            }
        }

        let (utilization, vram_used, vram_free) = query_ioreg();
        *cache = Some(PerfStats { utilization, vram_used, vram_free, updated_at: Instant::now() });
        (utilization, vram_used, vram_free)
    }
}

fn detect_gpu_name() -> Option<String> {
    let output = Command::new("system_profiler")
        .args(["SPDisplaysDataType"])
        .output()
        .ok()?;

    let text = String::from_utf8_lossy(&output.stdout);
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("Chipset Model:") {
            return Some(trimmed["Chipset Model:".len()..].trim().to_string());
        }
    }
    None
}

fn query_ioreg() -> (Option<f32>, Option<u64>, Option<u64>) {
    let output = match Command::new("ioreg")
        .args(["-r", "-c", "IOAccelerator", "-w", "0"])
        .output()
    {
        Ok(o) => o,
        Err(_) => return (None, None, None),
    };

    let text = String::from_utf8_lossy(&output.stdout);
    let mut utilization = None;
    let mut vram_used = None;
    let mut vram_free = None;

    for line in text.lines() {
        if line.contains("PerformanceStatistics") {
            if let Some(v) = extract_stat(line, "Device Utilization %") {
                utilization = Some(v as f32);
            }
            if let Some(v) = extract_stat(line, "vramUsedBytes") {
                vram_used = Some(v);
            }
            if let Some(v) = extract_stat(line, "vramFreeBytes") {
                vram_free = Some(v);
            }
        }
    }

    (utilization, vram_used, vram_free)
}

fn extract_stat(line: &str, key: &str) -> Option<u64> {
    let pattern = format!("\"{}\"=", key);
    let start = line.find(&pattern)? + pattern.len();
    let rest = &line[start..];
    let end = rest.find(|c: char| !c.is_ascii_digit()).unwrap_or(rest.len());
    rest[..end].parse().ok()
}

impl GPUBackend for AppleBackend {
    fn name(&self) -> String {
        self.name.clone()
    }

    fn vendor(&self) -> String {
        "Apple".to_string()
    }

    fn temperature(&self) -> Option<f32> {
        None
    }

    fn utilization(&self) -> Option<f32> {
        self.perf_stats().0
    }

    fn memory_used(&self) -> Option<u64> {
        self.perf_stats().1
    }

    fn memory_total(&self) -> Option<u64> {
        let (_, used, free) = self.perf_stats();
        match (used, free) {
            (Some(u), Some(f)) => Some(u + f),
            _ => None,
        }
    }

    fn power_draw(&self) -> Option<f32> {
        None
    }

    fn power_limit(&self) -> Option<f32> {
        None
    }

    fn clock_speed(&self) -> Option<u32> {
        None
    }

    fn memory_clock(&self) -> Option<u32> {
        None
    }

    fn fan_speed(&self) -> Option<u32> {
        None
    }

    fn processes(&self) -> Vec<GPUProcess> {
        Vec::new()
    }
}
