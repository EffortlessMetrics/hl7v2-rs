//! Performance monitoring utilities for the CLI

use std::collections::HashMap;
use std::time::Instant;
use sysinfo::System;

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
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "retained for report expansion; debt is tracked in policy/clippy-debt.toml"
        )
    )]
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

    let Ok(pid) = sysinfo::get_current_pid() else {
        return MemoryInfo {
            resident_set_size: None,
            virtual_memory_size: None,
        };
    };

    if let Some(process) = sys.process(pid) {
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

    let cpus = sys.cpus();
    let Some(cpu_count) = u32::try_from(cpus.len()).ok().filter(|count| *count > 0) else {
        return CpuInfo {
            cpu_usage_percent: None,
        };
    };
    let total_usage: f64 = cpus.iter().map(|cpu| f64::from(cpu.cpu_usage())).sum();
    let cpu_usage = total_usage / f64::from(cpu_count);

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

#[cfg(test)]
mod tests {
    use super::{
        CpuInfo, MemoryInfo, PerformanceMonitor, SystemInfo, get_cpu_info, get_memory_info,
        get_system_info,
    };
    use std::time::Duration;

    #[test]
    fn performance_monitor_new_elapsed_is_non_negative() {
        let monitor = PerformanceMonitor::new();
        let elapsed = monitor.elapsed();
        assert!(elapsed.as_nanos() < u128::MAX);
    }

    #[test]
    fn performance_monitor_records_and_returns_metric() {
        let mut monitor = PerformanceMonitor::new();
        let duration = Duration::from_millis(42);
        monitor.record_metric("parse", duration);

        assert_eq!(monitor.get_metric("parse"), Some(duration));
    }

    #[test]
    fn performance_monitor_records_multiple_distinct_keys() {
        let mut monitor = PerformanceMonitor::new();
        let a = Duration::from_millis(10);
        let b = Duration::from_millis(20);
        monitor.record_metric("a", a);
        monitor.record_metric("b", b);

        assert_eq!(monitor.get_metric("a"), Some(a));
        assert_eq!(monitor.get_metric("b"), Some(b));
        assert_eq!(monitor.get_metrics().len(), 2);
    }

    #[test]
    fn performance_monitor_record_metric_overwrites_existing_key() {
        let mut monitor = PerformanceMonitor::new();
        monitor.record_metric("step", Duration::from_millis(1));
        monitor.record_metric("step", Duration::from_millis(2));

        assert_eq!(monitor.get_metric("step"), Some(Duration::from_millis(2)));
        assert_eq!(monitor.get_metrics().len(), 1);
    }

    #[test]
    fn performance_monitor_get_metric_returns_none_for_unknown_key() {
        let monitor = PerformanceMonitor::new();
        assert!(monitor.get_metric("missing").is_none());
    }

    #[test]
    fn performance_monitor_get_metrics_reflects_record_count() {
        let mut monitor = PerformanceMonitor::new();
        assert!(monitor.get_metrics().is_empty());

        monitor.record_metric("one", Duration::from_micros(1));
        monitor.record_metric("two", Duration::from_micros(2));
        monitor.record_metric("three", Duration::from_micros(3));

        let metrics = monitor.get_metrics();
        assert_eq!(metrics.len(), 3);
        assert!(metrics.contains_key("one"));
        assert!(metrics.contains_key("two"));
        assert!(metrics.contains_key("three"));
    }

    #[test]
    fn performance_monitor_clone_preserves_metrics() {
        let mut monitor = PerformanceMonitor::new();
        monitor.record_metric("a", Duration::from_millis(5));
        let cloned = monitor.clone();

        assert_eq!(cloned.get_metric("a"), Some(Duration::from_millis(5)));
        let debug_repr = format!("{monitor:?}");
        assert!(debug_repr.contains("PerformanceMonitor"));
    }

    #[test]
    fn memory_info_debug_and_clone_round_trip() {
        let info = MemoryInfo {
            resident_set_size: Some(0),
            virtual_memory_size: Some(0),
        };
        let cloned = info.clone();
        assert_eq!(cloned.resident_set_size, Some(0));
        assert_eq!(cloned.virtual_memory_size, Some(0));

        let debug_repr = format!("{info:?}");
        assert!(debug_repr.contains("MemoryInfo"));
    }

    #[test]
    fn cpu_info_debug_and_clone_round_trip() {
        let info = CpuInfo {
            cpu_usage_percent: Some(0.0),
        };
        let cloned = info.clone();
        assert_eq!(cloned.cpu_usage_percent, Some(0.0));

        let debug_repr = format!("{info:?}");
        assert!(debug_repr.contains("CpuInfo"));
    }

    #[test]
    fn system_info_debug_and_clone_round_trip() {
        let info = SystemInfo {
            memory: MemoryInfo {
                resident_set_size: Some(1),
                virtual_memory_size: Some(2),
            },
            cpu: CpuInfo {
                cpu_usage_percent: Some(3.0),
            },
            total_memory: 4,
            used_memory: 5,
        };
        let cloned = info.clone();
        assert_eq!(cloned.total_memory, 4);
        assert_eq!(cloned.used_memory, 5);

        let debug_repr = format!("{info:?}");
        assert!(debug_repr.contains("SystemInfo"));
    }

    #[test]
    fn get_memory_info_returns_value_without_panicking() {
        let info = get_memory_info();
        let _ = info.resident_set_size;
        let _ = info.virtual_memory_size;
    }

    #[test]
    fn get_cpu_info_returns_value_without_panicking() {
        let info = get_cpu_info();
        let _ = info.cpu_usage_percent;
    }

    #[test]
    fn get_system_info_returns_value_without_panicking() {
        let info = get_system_info();
        let _ = info.total_memory;
        let _ = info.used_memory;
        let _ = info.memory.resident_set_size;
        let _ = info.cpu.cpu_usage_percent;
    }

    #[test]
    fn benchmark_macro_propagates_result_and_records_duration() {
        let (result, duration) = crate::benchmark!("compute", {
            let mut sum: u64 = 0;
            for i in 0..1000u64 {
                sum = sum.wrapping_add(i);
            }
            sum
        });

        assert_eq!(result, (0..1000u64).sum::<u64>());
        assert!(duration.as_nanos() < u128::MAX);
    }
}
