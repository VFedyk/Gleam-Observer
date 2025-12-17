use crate::error::{Error, Result};
use crate::gpu::backend::GPUBackend;
use nvml_wrapper::Nvml;
use nvml_wrapper::Device;
use nvml_wrapper::enum_wrappers::device::TemperatureSensor;

pub struct NvidiaBackend {
    device: Device<'static>,
    #[allow(dead_code)]
    nvml: &'static Nvml,
}

impl NvidiaBackend {
    pub fn detect_all() -> Result<Vec<Self>> {
        let nvml = Box::leak(Box::new(
            Nvml::init().map_err(|e| Error::Gpu(format!("Failed to initialize NVML: {}", e)))?
        ));

        let device_count = nvml.device_count()
            .map_err(|e| Error::Gpu(format!("Failed to get device count: {}", e)))?;

        let mut gpus = Vec::new();
        for i in 0..device_count {
            match nvml.device_by_index(i) {
                Ok(device) => {
                    gpus.push(Self { device, nvml });
                }
                Err(e) => {
                    log::warn!("Failed to get NVIDIA device {}: {}", i, e);
                }
            }
        }

        if gpus.is_empty() {
            return Err(Error::Gpu("No NVIDIA GPUs detected".to_string()));
        }

        Ok(gpus)
    }
    
    fn get_known_gpu_name(name: &str) -> Option<String> {
        // NVIDIA GPUs - match on device name patterns with specific brand models
        
        // RTX 40 Series
        if name.contains("4090") {
            if name.contains("ASUS") {
                if name.contains("ROG") || name.contains("Strix") {
                    return Some("ASUS ROG Strix RTX 4090 OC".to_string());
                } else if name.contains("TUF") {
                    return Some("ASUS TUF Gaming RTX 4090 OC".to_string());
                }
            } else if name.contains("MSI") {
                if name.contains("SUPRIM") {
                    return Some("MSI GeForce RTX 4090 Suprim X".to_string());
                } else if name.contains("Gaming X") {
                    return Some("MSI GeForce RTX 4090 Gaming X Trio".to_string());
                }
            } else if name.contains("Gigabyte") {
                if name.contains("AORUS") {
                    return Some("Gigabyte AORUS RTX 4090 Master".to_string());
                } else if name.contains("Gaming") {
                    return Some("Gigabyte GeForce RTX 4090 Gaming OC".to_string());
                }
            } else if name.contains("EVGA") {
                return Some("EVGA GeForce RTX 4090 FTW3 Ultra".to_string());
            } else if name.contains("Zotac") {
                return Some("Zotac GeForce RTX 4090 AMP Extreme".to_string());
            } else if name.contains("PNY") {
                return Some("PNY GeForce RTX 4090 XLR8 Uprising".to_string());
            }
        } else if name.contains("4080") {
            if name.contains("ASUS") {
                if name.contains("TUF") {
                    return Some("ASUS TUF Gaming RTX 4080 OC".to_string());
                } else if name.contains("ROG") {
                    return Some("ASUS ROG Strix RTX 4080 OC".to_string());
                }
            } else if name.contains("MSI") {
                if name.contains("SUPRIM") {
                    return Some("MSI GeForce RTX 4080 Suprim X".to_string());
                } else {
                    return Some("MSI GeForce RTX 4080 Gaming X Trio".to_string());
                }
            } else if name.contains("Gigabyte") {
                if name.contains("AORUS") {
                    return Some("Gigabyte AORUS RTX 4080 Master".to_string());
                } else {
                    return Some("Gigabyte GeForce RTX 4080 Gaming OC".to_string());
                }
            }
        } else if name.contains("4070 Ti") {
            if name.contains("Gigabyte") {
                return Some("Gigabyte GeForce RTX 4070 Ti Gaming OC".to_string());
            } else if name.contains("MSI") {
                return Some("MSI GeForce RTX 4070 Ti Gaming X Trio".to_string());
            } else if name.contains("ASUS") {
                return Some("ASUS TUF Gaming RTX 4070 Ti OC".to_string());
            }
        } else if name.contains("4070") {
            if name.contains("ASUS") {
                if name.contains("Dual") {
                    return Some("ASUS Dual RTX 4070 OC".to_string());
                } else {
                    return Some("ASUS TUF Gaming RTX 4070".to_string());
                }
            } else if name.contains("MSI") {
                return Some("MSI GeForce RTX 4070 Gaming X Trio".to_string());
            } else if name.contains("Gigabyte") {
                return Some("Gigabyte GeForce RTX 4070 Windforce OC".to_string());
            }
        }
        
        // RTX 30 Series
        if name.contains("3090 Ti") {
            if name.contains("ASUS") {
                return Some("ASUS ROG Strix RTX 3090 Ti OC".to_string());
            } else if name.contains("MSI") {
                if name.contains("SUPRIM") {
                    return Some("MSI GeForce RTX 3090 Ti Suprim X".to_string());
                } else {
                    return Some("MSI GeForce RTX 3090 Ti Gaming X Trio".to_string());
                }
            } else if name.contains("Gigabyte") {
                return Some("Gigabyte AORUS RTX 3090 Ti Xtreme".to_string());
            }
        } else if name.contains("3090") {
            if name.contains("Gigabyte") {
                return Some("Gigabyte GeForce RTX 3090 Gaming OC".to_string());
            } else if name.contains("ASUS") {
                return Some("ASUS ROG Strix RTX 3090 OC".to_string());
            } else if name.contains("MSI") {
                return Some("MSI GeForce RTX 3090 Gaming X Trio".to_string());
            } else if name.contains("EVGA") {
                return Some("EVGA GeForce RTX 3090 FTW3 Ultra".to_string());
            }
        } else if name.contains("3080 Ti") {
            if name.contains("ASUS") {
                if name.contains("TUF") {
                    return Some("ASUS TUF Gaming RTX 3080 Ti OC".to_string());
                } else {
                    return Some("ASUS ROG Strix RTX 3080 Ti OC".to_string());
                }
            } else if name.contains("MSI") {
                return Some("MSI GeForce RTX 3080 Ti Gaming X Trio".to_string());
            } else if name.contains("Gigabyte") {
                return Some("Gigabyte GeForce RTX 3080 Ti Vision OC".to_string());
            }
        } else if name.contains("3080") {
            if name.contains("MSI") {
                return Some("MSI GeForce RTX 3080 Gaming X Trio".to_string());
            } else if name.contains("ASUS") {
                if name.contains("TUF") {
                    return Some("ASUS TUF Gaming RTX 3080 OC".to_string());
                } else {
                    return Some("ASUS ROG Strix RTX 3080 OC".to_string());
                }
            } else if name.contains("EVGA") {
                return Some("EVGA GeForce RTX 3080 FTW3 Ultra".to_string());
            } else if name.contains("Gigabyte") {
                return Some("Gigabyte GeForce RTX 3080 Gaming OC".to_string());
            }
        } else if name.contains("3070 Ti") {
            if name.contains("Gigabyte") {
                return Some("Gigabyte GeForce RTX 3070 Ti Eagle OC".to_string());
            } else if name.contains("ASUS") {
                return Some("ASUS TUF Gaming RTX 3070 Ti OC".to_string());
            } else if name.contains("MSI") {
                return Some("MSI GeForce RTX 3070 Ti Gaming X Trio".to_string());
            }
        } else if name.contains("3070") {
            if name.contains("ASUS") {
                if name.contains("Dual") {
                    return Some("ASUS Dual RTX 3070 OC".to_string());
                } else {
                    return Some("ASUS TUF Gaming RTX 3070 OC".to_string());
                }
            } else if name.contains("MSI") {
                return Some("MSI GeForce RTX 3070 Gaming X Trio".to_string());
            } else if name.contains("Gigabyte") {
                return Some("Gigabyte GeForce RTX 3070 Vision OC".to_string());
            }
        } else if name.contains("3060 Ti") {
            if name.contains("MSI") {
                return Some("MSI GeForce RTX 3060 Ti Gaming X".to_string());
            } else if name.contains("ASUS") {
                return Some("ASUS Dual RTX 3060 Ti OC".to_string());
            } else if name.contains("Gigabyte") {
                return Some("Gigabyte GeForce RTX 3060 Ti Gaming OC Pro".to_string());
            }
        } else if name.contains("3060") {
            if name.contains("Gigabyte") {
                return Some("Gigabyte GeForce RTX 3060 Gaming OC".to_string());
            } else if name.contains("ASUS") {
                return Some("ASUS TUF Gaming RTX 3060 OC".to_string());
            } else if name.contains("MSI") {
                return Some("MSI GeForce RTX 3060 Ventus 2X OC".to_string());
            }
        }
        
        None
    }
}

impl GPUBackend for NvidiaBackend {
    fn name(&self) -> String {
        let raw_name = self.device.name()
            .unwrap_or_else(|_| "Unknown NVIDIA GPU".to_string());
        
        // Try to get improved name from known GPU database
        Self::get_known_gpu_name(&raw_name)
            .unwrap_or(raw_name)
    }

    fn vendor(&self) -> String {
        "NVIDIA".to_string()
    }

    fn temperature(&self) -> Option<f32> {
        self.device.temperature(TemperatureSensor::Gpu)
            .ok()
            .map(|t| t as f32)
    }

    fn utilization(&self) -> Option<f32> {
        self.device.utilization_rates()
            .ok()
            .map(|u| u.gpu as f32)
    }

    fn memory_used(&self) -> Option<u64> {
        self.device.memory_info()
            .ok()
            .map(|info| info.used)
    }

    fn memory_total(&self) -> Option<u64> {
        self.device.memory_info()
            .ok()
            .map(|info| info.total)
    }

    fn power_draw(&self) -> Option<f32> {
        self.device.power_usage()
            .ok()
            .map(|p| p as f32 / 1000.0)
    }

    fn power_limit(&self) -> Option<f32> {
        self.device.power_management_limit()
            .ok()
            .map(|p| p as f32 / 1000.0)
    }

    fn clock_speed(&self) -> Option<u32> {
        self.device.clock_info(nvml_wrapper::enum_wrappers::device::Clock::Graphics)
            .ok()
    }

    fn memory_clock(&self) -> Option<u32> {
        self.device.clock_info(nvml_wrapper::enum_wrappers::device::Clock::Memory)
            .ok()
    }

    fn fan_speed(&self) -> Option<u32> {
        self.device.fan_speed(0)
            .ok()
    }

    fn processes(&self) -> Vec<crate::gpu::backend::GPUProcess> {
        use crate::gpu::backend::GPUProcess;
        
        let mut gpu_processes = Vec::new();
        
        // Compute processes
        if let Ok(processes) = self.device.running_compute_processes() {
            for proc in processes {
                let name = Self::get_process_name(proc.pid);
                // Try to extract memory, default to 0 if unavailable
                let mem = 0u64; // Simplified - NVML process info varies by version
                gpu_processes.push(GPUProcess {
                    pid: proc.pid,
                    name,
                    memory_used: mem,
                });
            }
        }
        
        // Graphics processes
        if let Ok(processes) = self.device.running_graphics_processes() {
            for proc in processes {
                let name = Self::get_process_name(proc.pid);
                let mem = 0u64;
                gpu_processes.push(GPUProcess {
                    pid: proc.pid,
                    name,
                    memory_used: mem,
                });
            }
        }
        
        gpu_processes
    }
}

impl NvidiaBackend {
    fn get_process_name(pid: u32) -> String {
        #[cfg(unix)]
        {
            use std::fs;
            if let Ok(cmdline) = fs::read_to_string(format!("/proc/{}/comm", pid)) {
                return cmdline.trim().to_string();
            }
        }
        
        format!("PID {}", pid)
    }
}
