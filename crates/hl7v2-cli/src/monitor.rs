//! Performance monitoring utilities for the CLI

use std::time::Instant;
use std::collections::HashMap;
use sysinfo::{System, CpuExt, ProcessExt, SystemExt};

/// Performance metrics collector
#[derive(Debug, Clone)]
pub struct PerformanceMonitor {
    start_time: Instant,
    metrics: HashMap<String, std::time::Duration>,
}

impl PerformanceMonitor {
    /// Create a new performance monitor
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            metrics: HashMap::new(),
        }
    }
    
    /// Record a metric
    pub fn record_metric(&mut self, name: &str, duration: std::time::Duration) {
        self.metrics.insert(name.to_string(), duration);
    }
    
    /// Get elapsed time since creation
    pub fn elapsed(&self) -> std::time::Duration {
        self.start_time.elapsed()
    }
    
    /// Get a specific metric
    pub fn get_metric(&self, name: &str) -> Option<std::time::Duration> {
        self.metrics.get(name).copied()
    }
    
    /// Get all metrics
    pub fn get_metrics(&self) -> &HashMap<String, std::time::Duration> {
        &self.metrics
    }
}

/// Simple benchmarking macro
#[macro_export]
macro_rules! benchmark {
    ($name:expr, $block:block) => {{
        let start = std::time::Instant::now();
        let result = $block;
        let duration = start.elapsed();
        (result, duration)
    }};
}

/// Memory usage information
#[derive(Debug, Clone)]
pub struct MemoryInfo {
    pub resident_set_size: Option<u64>,
    pub virtual_memory_size: Option<u64>,
}

/// Get current memory usage
pub fn get_memory_info() -> MemoryInfo {
    let mut sys = System::new_all();
    sys.refresh_all();
    
    if let Some(process) = sys.process(sysinfo::get_current_pid().unwrap()) {
        MemoryInfo {
            resident_set_size: Some(process.memory()),
            virtual_memory_size: Some(process.virtual_memory()),
        }
    } else {
        MemoryInfo {
            resident_set_size: None,
            virtual_memory_size: None,
        }
    }
}

/// CPU usage information
#[derive(Debug, Clone)]
pub struct CpuInfo {
    pub cpu_usage_percent: Option<f64>,
}

/// Get current CPU usage
pub fn get_cpu_info() -> CpuInfo {
    let mut sys = System::new_all();
    sys.refresh_all();
    
    let cpu_usage: f64 = sys.cpus().iter().map(|cpu| cpu.cpu_usage()).sum::<f32>() as f64 / sys.cpus().len() as f64;
    
    CpuInfo {
        cpu_usage_percent: Some(cpu_usage),
    }
}

/// System information
#[derive(Debug, Clone)]
pub struct SystemInfo {
    pub memory: MemoryInfo,
    pub cpu: CpuInfo,
    pub total_memory: u64,
    pub used_memory: u64,
}

/// Get comprehensive system information
pub fn get_system_info() -> SystemInfo {
    let mut sys = System::new_all();
    sys.refresh_all();
    
    let memory_info = get_memory_info();
    let cpu_info = get_cpu_info();
    
    SystemInfo {
        memory: memory_info,
        cpu: cpu_info,
        total_memory: sys.total_memory(),
        used_memory: sys.used_memory(),
    }
}

/// Format bytes into a human-readable string
pub fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    const TB: u64 = GB * 1024;

    if bytes < KB {
        format!("{} B", bytes)
    } else if bytes < MB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else if bytes < GB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes < TB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else {
        format!("{:.2} TB", bytes as f64 / TB as f64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(1023), "1023 B");
        assert_eq!(format_size(1024), "1.00 KB");
        assert_eq!(format_size(1536), "1.50 KB");
        assert_eq!(format_size(1024 * 1024), "1.00 MB");
        assert_eq!(format_size(1024 * 1024 * 1024), "1.00 GB");
        assert_eq!(format_size(1024 * 1024 * 1024 * 1024), "1.00 TB");
    }
}
