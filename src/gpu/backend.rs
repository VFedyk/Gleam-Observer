pub trait GPUBackend: Send + Sync {
    fn name(&self) -> String;
    fn vendor(&self) -> String;
    fn temperature(&self) -> Option<f32>;
    fn utilization(&self) -> Option<f32>;
    fn memory_used(&self) -> Option<u64>;
    fn memory_total(&self) -> Option<u64>;
    fn power_draw(&self) -> Option<f32>;
    fn power_limit(&self) -> Option<f32>;
    fn clock_speed(&self) -> Option<u32>;
    fn memory_clock(&self) -> Option<u32>;
    fn fan_speed(&self) -> Option<u32>;
    fn processes(&self) -> Vec<GPUProcess>;
}

#[derive(Debug, Clone)]
pub struct GPUProcess {
    pub pid: u32,
    pub name: String,
    pub memory_used: u64, // bytes
}

pub struct GPUManager {
    backends: Vec<Box<dyn GPUBackend>>,
}

impl GPUManager {
    pub fn new() -> Self {
        let mut backends: Vec<Box<dyn GPUBackend>> = Vec::new();

        #[cfg(feature = "nvidia")]
        {
            use super::nvidia::NvidiaBackend;
            if let Ok(nvidia_gpus) = NvidiaBackend::detect_all() {
                for gpu in nvidia_gpus {
                    backends.push(Box::new(gpu));
                }
            }
        }

        #[cfg(all(target_os = "linux", feature = "amd"))]
        {
            use super::amd::AmdBackend;
            if let Ok(amd_gpus) = AmdBackend::detect_all() {
                for gpu in amd_gpus {
                    backends.push(Box::new(gpu));
                }
            }
        }

        #[cfg(feature = "intel")]
        {
            use super::intel::IntelBackend;
            if let Ok(intel_gpus) = IntelBackend::detect_all() {
                for gpu in intel_gpus {
                    backends.push(Box::new(gpu));
                }
            }
        }

        #[cfg(all(target_os = "macos", feature = "apple-gpu"))]
        {
            use super::apple::AppleBackend;
            if let Ok(apple_gpus) = AppleBackend::detect_all() {
                for gpu in apple_gpus {
                    backends.push(Box::new(gpu));
                }
            }
        }

        Self { backends }
    }

    pub fn gpus(&self) -> &[Box<dyn GPUBackend>] {
        &self.backends
    }

    pub fn gpu_count(&self) -> usize {
        self.backends.len()
    }

    pub fn has_gpus(&self) -> bool {
        !self.backends.is_empty()
    }

    pub fn get_info(&self) -> Vec<GPUInfo> {
        self.backends.iter()
            .enumerate()
            .map(|(i, gpu)| {
                let power_draw = gpu.power_draw();
                let power_limit = gpu.power_limit();
                let utilization = gpu.utilization();
                
                // Calculate power efficiency (utilization per watt)
                let power_efficiency = if let (Some(util), Some(power)) = (utilization, power_draw) {
                    if power > 0.0 {
                        Some(util / power)
                    } else {
                        None
                    }
                } else {
                    None
                };
                
                GPUInfo {
                    id: i,
                    name: gpu.name(),
                    vendor: gpu.vendor(),
                    temperature: gpu.temperature(),
                    utilization,
                    memory_used: gpu.memory_used(),
                    memory_total: gpu.memory_total(),
                    power_draw,
                    power_limit,
                    power_efficiency,
                    clock_speed: gpu.clock_speed(),
                    memory_clock: gpu.memory_clock(),
                    fan_speed: gpu.fan_speed(),
                    processes: gpu.processes(),
                }
            })
            .collect()
    }
}

impl Default for GPUManager {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct GPUInfo {
    pub id: usize,
    pub name: String,
    pub vendor: String,
    pub temperature: Option<f32>,
    pub utilization: Option<f32>,
    pub memory_used: Option<u64>,
    pub memory_total: Option<u64>,
    pub power_draw: Option<f32>,
    pub power_limit: Option<f32>,
    pub power_efficiency: Option<f32>,
    pub clock_speed: Option<u32>,
    pub memory_clock: Option<u32>,
    pub fan_speed: Option<u32>,
    pub processes: Vec<GPUProcess>,
}

impl GPUInfo {
    pub fn memory_usage_percent(&self) -> Option<f32> {
        match (self.memory_used, self.memory_total) {
            (Some(used), Some(total)) if total > 0 => {
                Some((used as f32 / total as f32) * 100.0)
            }
            _ => None,
        }
    }
}
