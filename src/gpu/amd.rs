#[cfg(target_os = "linux")]
use std::fs;
#[cfg(target_os = "linux")]
use std::path::PathBuf;

use crate::error::{Error, Result};
use crate::gpu::backend::GPUBackend;

#[cfg(target_os = "linux")]
pub struct AmdBackend {
    hwmon_path: PathBuf,
    device_path: PathBuf,
    name: String,
}

#[cfg(target_os = "linux")]
impl AmdBackend {
    pub fn detect_all() -> Result<Vec<Self>> {
        let mut gpus = Vec::new();

        let cards = match fs::read_dir("/sys/class/drm") {
            Ok(entries) => entries,
            Err(_) => return Err(Error::Gpu("Failed to read /sys/class/drm".to_string())),
        };

        for entry in cards.flatten() {
            let path = entry.path();
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

            // Skip display connectors (card1-DP-1, card1-HDMI-A-1, etc.)
            // Only process main cards (card0, card1, etc.)
            if !name.starts_with("card") || name.contains('-') {
                continue;
            }

            let device_path = path.join("device");

            if !device_path.exists() {
                continue;
            }

            if let Ok(vendor) = fs::read_to_string(device_path.join("vendor")) {
                if vendor.trim() == "0x1002" {
                    if let Some(hwmon_path) = Self::find_hwmon(&device_path) {
                        let name = Self::read_gpu_name(&device_path);
                        gpus.push(Self {
                            hwmon_path,
                            device_path,
                            name,
                        });
                    }
                }
            }
        }

        if gpus.is_empty() {
            return Err(Error::Gpu("No AMD GPUs detected".to_string()));
        }

        Ok(gpus)
    }

    fn find_hwmon(device_path: &PathBuf) -> Option<PathBuf> {
        let hwmon_dir = device_path.join("hwmon");
        if let Ok(entries) = fs::read_dir(&hwmon_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Ok(name) = fs::read_to_string(path.join("name")) {
                    if name.trim() == "amdgpu" {
                        return Some(path);
                    }
                }
            }
        }
        None
    }

    fn read_gpu_name(device_path: &PathBuf) -> String {
        // Try sysfs first
        if let Ok(name) = fs::read_to_string(device_path.join("product_name")) {
            let trimmed = name.trim();
            if !trimmed.is_empty() {
                return trimmed.to_string();
            }
        }

        // Get device ID for known GPU mapping
        let device_id = Self::read_device_id(device_path);

        // Fallback to lspci with machine-readable format
        if let Ok(pci_slot) = fs::read_link(device_path) {
            if let Some(slot) = pci_slot.file_name().and_then(|s| s.to_str()) {
                if let Ok(output) = std::process::Command::new("lspci")
                    .args(&["-vmm", "-s", slot])
                    .output()
                {
                    if let Ok(text) = String::from_utf8(output.stdout) {
                        let mut device_name = None;
                        let mut sdevice_name = None;
                        let mut svendor = None;

                        for line in text.lines() {
                            if line.starts_with("Device:") {
                                device_name = Some(line.trim_start_matches("Device:").trim());
                            } else if line.starts_with("SDevice:") {
                                sdevice_name = Some(line.trim_start_matches("SDevice:").trim());
                            } else if line.starts_with("SVendor:") {
                                svendor = Some(line.trim_start_matches("SVendor:").trim());
                            }
                        }

                        // Apply known GPU mappings for proper naming
                        if let (Some(dev_id), Some(vendor)) = (&device_id, svendor) {
                            if let Some(proper_name) = Self::get_known_gpu_name(dev_id, vendor) {
                                return proper_name;
                            }
                        }

                        // Prefer subsystem device name (more specific)
                        if let Some(name) = sdevice_name {
                            if !name.is_empty() {
                                return name.to_string();
                            }
                        }

                        // Fallback to device name, extract from brackets
                        if let Some(name) = device_name {
                            if let Some(bracketed) = name.split('[').nth(1) {
                                if let Some(extracted) = bracketed.split(']').next() {
                                    return extracted.trim().to_string();
                                }
                            }
                            return name.to_string();
                        }
                    }
                }
            }
        }

        "Unknown AMD GPU".to_string()
    }

    fn read_device_id(device_path: &PathBuf) -> Option<String> {
        if let Ok(pci_slot) = fs::read_link(device_path) {
            if let Some(slot) = pci_slot.file_name().and_then(|s| s.to_str()) {
                if let Ok(output) = std::process::Command::new("lspci")
                    .args(&["-n", "-s", slot])
                    .output()
                {
                    if let Ok(text) = String::from_utf8(output.stdout) {
                        // Parse "08:00.0 0300: 1002:73df (rev c1)"
                        if let Some(ids) = text.split_whitespace().nth(2) {
                            if let Some(device) = ids.split(':').nth(1) {
                                return Some(device.to_string());
                            }
                        }
                    }
                }
            }
        }
        None
    }

    fn get_known_gpu_name(device_id: &str, vendor: &str) -> Option<String> {
        // Sapphire GPUs
        if vendor.contains("Sapphire") {
            match device_id {
                "73df" => return Some("Sapphire RX 6700 XT Nitro".to_string()),
                "73ef" => return Some("Sapphire RX 6650 XT Nitro".to_string()),
                "73ff" => return Some("Sapphire RX 6600 XT Nitro".to_string()),
                "7340" => return Some("Sapphire NITRO+ Radeon RX 7900 XTX Vapor-X".to_string()),
                "744c" => return Some("Sapphire NITRO+ Radeon RX 7900 XT".to_string()),
                "747e" => return Some("Sapphire NITRO+ Radeon RX 7800 XT".to_string()),
                "7480" => return Some("Sapphire NITRO+ Radeon RX 7700 XT".to_string()),
                _ => {}
            }
        }

        // ASUS GPUs
        if vendor.contains("ASUSTeK") || vendor.contains("ASUS") {
            match device_id {
                "73df" => return Some("ASUS ROG Strix RX 6700 XT OC".to_string()),
                "73ef" => return Some("ASUS TUF Gaming RX 6650 XT OC".to_string()),
                "73ff" => return Some("ASUS ROG Strix RX 6600 XT OC".to_string()),
                "7340" => return Some("ASUS ROG Strix RX 7900 XTX OC".to_string()),
                "744c" => return Some("ASUS TUF Gaming RX 7900 XT OC".to_string()),
                "747e" => return Some("ASUS TUF Gaming RX 7800 XT OC".to_string()),
                "7480" => return Some("ASUS Dual RX 7700 XT OC".to_string()),
                _ => {}
            }
        }

        // MSI GPUs
        if vendor.contains("Micro-Star") || vendor.contains("MSI") {
            match device_id {
                "73df" => return Some("MSI Radeon RX 6700 XT Gaming X".to_string()),
                "73ef" => return Some("MSI Radeon RX 6650 XT Mech 2X OC".to_string()),
                "73ff" => return Some("MSI Radeon RX 6600 XT Gaming X".to_string()),
                "7340" => return Some("MSI Radeon RX 7900 XTX Gaming Trio".to_string()),
                "744c" => return Some("MSI Radeon RX 7900 XT Gaming Trio".to_string()),
                "747e" => return Some("MSI Radeon RX 7800 XT Gaming Trio".to_string()),
                "7480" => return Some("MSI Radeon RX 7700 XT Gaming X".to_string()),
                _ => {}
            }
        }

        // Gigabyte GPUs
        if vendor.contains("Gigabyte") {
            match device_id {
                "73df" => return Some("Gigabyte Radeon RX 6700 XT Gaming OC".to_string()),
                "73ef" => return Some("Gigabyte Radeon RX 6650 XT Eagle".to_string()),
                "73ff" => return Some("Gigabyte Radeon RX 6600 XT Gaming OC Pro".to_string()),
                "7340" => return Some("Gigabyte Radeon RX 7900 XTX Gaming OC".to_string()),
                "744c" => return Some("Gigabyte Radeon RX 7900 XT Gaming OC".to_string()),
                "747e" => return Some("Gigabyte Radeon RX 7800 XT Gaming OC".to_string()),
                "7480" => return Some("Gigabyte Radeon RX 7700 XT Gaming OC".to_string()),
                _ => {}
            }
        }

        // XFX GPUs
        if vendor.contains("XFX") || vendor.contains("Pine Technology") {
            match device_id {
                "73df" => return Some("XFX Speedster MERC 319 RX 6700 XT".to_string()),
                "73ef" => return Some("XFX Speedster SWFT 210 RX 6650 XT".to_string()),
                "73ff" => return Some("XFX Speedster QICK 308 RX 6600 XT".to_string()),
                "7340" => return Some("XFX Speedster MERC 310 RX 7900 XTX".to_string()),
                "744c" => return Some("XFX Speedster MERC 310 RX 7900 XT".to_string()),
                "747e" => return Some("XFX Speedster MERC 319 RX 7800 XT".to_string()),
                "7480" => return Some("XFX Speedster QICK 319 RX 7700 XT".to_string()),
                _ => {}
            }
        }

        // PowerColor GPUs
        if vendor.contains("PowerColor") || vendor.contains("TUL Corporation") {
            match device_id {
                "73df" => return Some("PowerColor Red Devil RX 6700 XT".to_string()),
                "73ef" => return Some("PowerColor Fighter RX 6650 XT".to_string()),
                "73ff" => return Some("PowerColor Red Devil RX 6600 XT".to_string()),
                "7340" => return Some("PowerColor Red Devil RX 7900 XTX".to_string()),
                "744c" => return Some("PowerColor Red Devil RX 7900 XT".to_string()),
                "747e" => return Some("PowerColor Hellhound RX 7800 XT".to_string()),
                "7480" => return Some("PowerColor Hellhound RX 7700 XT".to_string()),
                _ => {}
            }
        }

        None
    }

    fn read_sysfs_value(&self, filename: &str) -> Option<String> {
        fs::read_to_string(self.hwmon_path.join(filename))
            .ok()
            .map(|s| s.trim().to_string())
    }
}

#[cfg(target_os = "linux")]
impl GPUBackend for AmdBackend {
    fn name(&self) -> String {
        self.name.clone()
    }

    fn vendor(&self) -> String {
        "AMD".to_string()
    }

    fn temperature(&self) -> Option<f32> {
        self.read_sysfs_value("temp1_input")
            .and_then(|s| s.parse::<i32>().ok())
            .map(|t| t as f32 / 1000.0)
    }

    fn utilization(&self) -> Option<f32> {
        fs::read_to_string(self.device_path.join("gpu_busy_percent"))
            .ok()
            .and_then(|s| s.trim().parse::<f32>().ok())
    }

    fn memory_used(&self) -> Option<u64> {
        fs::read_to_string(self.device_path.join("mem_info_vram_used"))
            .ok()
            .and_then(|s| s.trim().parse::<u64>().ok())
    }

    fn memory_total(&self) -> Option<u64> {
        fs::read_to_string(self.device_path.join("mem_info_vram_total"))
            .ok()
            .and_then(|s| s.trim().parse::<u64>().ok())
    }

    fn power_draw(&self) -> Option<f32> {
        self.read_sysfs_value("power1_average")
            .and_then(|s| s.parse::<u64>().ok())
            .map(|p| p as f32 / 1_000_000.0)
    }

    fn power_limit(&self) -> Option<f32> {
        self.read_sysfs_value("power1_cap")
            .and_then(|s| s.parse::<u64>().ok())
            .map(|v| v as f32 / 1_000_000.0) // µW to W
    }

    fn clock_speed(&self) -> Option<u32> {
        self.read_sysfs_value("freq1_input")
            .and_then(|s| s.parse::<u64>().ok())
            .map(|f| (f / 1_000_000) as u32)
    }

    fn memory_clock(&self) -> Option<u32> {
        self.read_sysfs_value("pp_dpm_mclk").and_then(|content| {
            content
                .lines()
                .find(|line| line.contains('*'))
                .and_then(|line| {
                    line.split(':')
                        .nth(1)?
                        .trim()
                        .split("Mhz")
                        .next()?
                        .trim()
                        .parse::<u32>()
                        .ok()
                })
        })
    }

    fn fan_speed(&self) -> Option<u32> {
        self.read_sysfs_value("fan1_input")
            .and_then(|s| s.parse::<u32>().ok())
    }

    fn processes(&self) -> Vec<crate::gpu::backend::GPUProcess> {
        Vec::new()
    }
}

#[cfg(not(target_os = "linux"))]
pub struct AmdBackend;

#[cfg(not(target_os = "linux"))]
impl AmdBackend {
    pub fn detect_all() -> Result<Vec<Self>> {
        Err(Error::Gpu(
            "AMD GPU monitoring not supported on this platform".to_string(),
        ))
    }
}

#[cfg(not(target_os = "linux"))]
impl GPUBackend for AmdBackend {
    fn name(&self) -> String {
        "Unsupported".to_string()
    }
    fn vendor(&self) -> String {
        "AMD".to_string()
    }
    fn temperature(&self) -> Option<f32> {
        None
    }
    fn utilization(&self) -> Option<f32> {
        None
    }
    fn memory_used(&self) -> Option<u64> {
        None
    }
    fn memory_total(&self) -> Option<u64> {
        None
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
    fn processes(&self) -> Vec<crate::gpu::backend::GPUProcess> {
        Vec::new()
    }
}
