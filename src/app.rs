use crate::config::Config;
use crate::error::Result;
use crate::metrics::SystemMetrics;
use crate::gpu::GPUManager;
use crate::alerts::{AlertDetector, Alert, Notifier};
use crate::history::MetricsHistory;
use crate::trends::{TrendAnalyzer, MetricTrend};
use crate::process::{ProcessTree, smart_kill, force_kill};
use crate::metrics::system::ProcessInfo as SystemProcessInfo;
use std::time::{SystemTime, UNIX_EPOCH};
use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ViewMode {
    Dashboard,
    Processes,
    History,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ProcessSortMode {
    Cpu,
    Memory,
    Name,
    Pid,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DialogMode {
    None,
    ConfirmKill,
    ConfirmTerminate,
    ProcessInfo,
}

pub struct App {
    pub config: Config,
    pub metrics: SystemMetrics,
    pub gpu: Option<GPUManager>,
    pub gpu_info_cache: Vec<crate::gpu::GPUInfo>, // Cached GPU data
    pub alert_detector: AlertDetector,
    pub notifier: Notifier,
    pub active_alerts: Vec<Alert>,
    pub history: MetricsHistory,
    pub trend_analyzer: TrendAnalyzer,
    pub active_trends: Vec<MetricTrend>,
    pub running: bool,
    pub paused: bool,
    pub view_mode: ViewMode,
    pub process_sort: ProcessSortMode,
    pub selected_process_index: usize,
    pub dialog_mode: DialogMode,
    pub playback_index: Option<usize>,
    
    // Tree view support
    pub tree_mode: bool,
    pub process_tree: ProcessTree,
    pub collapsed_pids: HashSet<u32>,
    
    // Filter support
    pub filter_mode: bool,
    pub filter_input: String,
    pub filtered_processes: Vec<SystemProcessInfo>,
    
    // Status message for user feedback
    pub status_message: Option<String>,
    pub status_message_time: Option<SystemTime>,
}

impl App {
    pub fn new(config: Config, enable_gpu: bool) -> Result<Self> {
        let gpu = if enable_gpu {
            let manager = GPUManager::new();
            if manager.has_gpus() {
                log::info!("GPU monitoring enabled: {} GPU(s) detected", manager.gpu_count());
                Some(manager)
            } else {
                log::warn!("GPU monitoring enabled but no GPUs detected");
                None
            }
        } else {
            log::info!("GPU monitoring disabled");
            None
        };

        let alert_detector = AlertDetector::new(config.alerts.clone());
        let gpu_count = if let Some(ref gpu_manager) = gpu {
            gpu_manager.gpu_count()
        } else {
            0
        };
        let notifier = Notifier::new(config.alerts.notifications_enabled);
        let trend_analyzer = TrendAnalyzer::new(config.trends.clone());
        let history = MetricsHistory::new(config.refresh.history_samples, gpu_count);
        
        Ok(Self {
            alert_detector,
            notifier,
            metrics: SystemMetrics::new(),
            gpu,
            gpu_info_cache: Vec::new(), // Start with empty cache
            active_alerts: Vec::new(),
            history,
            trend_analyzer,
            active_trends: Vec::new(),
            running: true,
            paused: false,
            view_mode: ViewMode::Dashboard,
            process_sort: ProcessSortMode::Cpu,
            selected_process_index: 0,
            dialog_mode: DialogMode::None,
            playback_index: None,
            tree_mode: false,
            process_tree: ProcessTree::new(),
            collapsed_pids: HashSet::new(),
            filter_mode: false,
            filter_input: String::new(),
            filtered_processes: Vec::new(),
            status_message: None,
            status_message_time: None,
            config,
        })
    }

    pub fn update(&mut self) -> Result<()> {
        if !self.paused {
            self.metrics.refresh();
            
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            
            let cpu_usage = self.metrics.global_cpu_usage();
            let mem_usage = self.metrics.memory_usage_percent();
            let swap_usage = self.metrics.swap_usage_percent();
            let swap_total = self.metrics.swap_total();
            // Update GPU cache - only here during update()
            self.gpu_info_cache = if let Some(ref gpu_manager) = self.gpu {
                gpu_manager.get_info()
            } else {
                Vec::new()
            };
            
            // Update history
            self.history.update(timestamp, cpu_usage, mem_usage, swap_usage, &self.gpu_info_cache);
            
            // Check alerts
            let alerts = self.alert_detector.check_alerts(
                cpu_usage,
                mem_usage,
                swap_usage,
                swap_total,
                &self.gpu_info_cache,
            );
            
            // Send notifications for critical/warning alerts
            for alert in &alerts {
                if self.alert_detector.should_notify(&alert.alert_type) {
                    self.notifier.send_alert(alert);
                }
            }
            
            self.active_alerts = alerts;
        }

        // Analyze trends
        if self.config.trends.enabled && !self.paused {
            self.active_trends = self.trend_analyzer.analyze_all(
                &self.history,
                &self.config.alerts
            );
        }
        
        // Rebuild process tree if in tree mode
        if self.tree_mode {
            self.rebuild_tree();
        }
        
        // Reapply filter if filter is active
        if self.filter_mode && !self.filter_input.is_empty() {
            self.apply_filter();
        }
        
        // Update status message (clear old ones)
        self.update_status_message();

        // Keep selection within current process bounds
        self.clamp_process_selection();

        Ok(())
    }

    pub fn quit(&mut self) {
        self.running = false;
    }

    pub fn toggle_pause(&mut self) {
        self.paused = !self.paused;
    }

    pub fn has_gpu(&self) -> bool {
        self.gpu.is_some()
    }

    pub fn has_alerts(&self) -> bool {
        !self.active_alerts.is_empty()
    }

    pub fn critical_alert_count(&self) -> usize {
        self.alert_detector.critical_count()
    }

    pub fn warning_alert_count(&self) -> usize {
        self.alert_detector.warning_count()
    }

    pub fn toggle_view(&mut self) {
        self.view_mode = match self.view_mode {
            ViewMode::Dashboard => ViewMode::Processes,
            ViewMode::Processes => ViewMode::Dashboard,
            ViewMode::History => ViewMode::Dashboard,
        };
    }

    pub fn enter_history_mode(&mut self) {
        self.view_mode = ViewMode::History;
        let history_len = self.history.cpu_usage.len();
        if history_len > 0 {
            self.playback_index = Some(history_len - 1);
        }
    }

    pub fn exit_history_mode(&mut self) {
        self.view_mode = ViewMode::Dashboard;
        self.playback_index = None;
    }

    pub fn playback_step_backward(&mut self) {
        if let Some(idx) = self.playback_index {
            if idx > 0 {
                self.playback_index = Some(idx - 1);
            }
        }
    }

    pub fn playback_step_forward(&mut self) {
        if let Some(idx) = self.playback_index {
            let max_len = self.history.cpu_usage.len();
            if idx + 1 < max_len {
                self.playback_index = Some(idx + 1);
            }
        }
    }

    pub fn cycle_sort(&mut self) {
        self.process_sort = match self.process_sort {
            ProcessSortMode::Cpu => ProcessSortMode::Memory,
            ProcessSortMode::Memory => ProcessSortMode::Name,
            ProcessSortMode::Name => ProcessSortMode::Pid,
            ProcessSortMode::Pid => ProcessSortMode::Cpu,
        };
    }

    pub fn move_selection_up(&mut self) {
        if self.selected_process_index > 0 {
            self.selected_process_index -= 1;
        }
    }

    pub fn move_selection_down(&mut self) {
        let max = self.process_count();
        if self.selected_process_index < max.saturating_sub(1) {
            self.selected_process_index += 1;
        }
    }

    pub fn get_selected_pid(&self) -> Option<u32> {
        let processes = self.current_processes();
        
        processes.get(self.selected_process_index).map(|p| p.pid)
    }

    /// Return the list of processes currently visible in the UI,
    /// respecting filters, the active sort order, and grouping threads
    /// under their thread-group ID (TGID) to avoid duplicate rows.
    pub fn current_processes(&self) -> Vec<SystemProcessInfo> {
        let base_list = if self.filter_mode && !self.filter_input.is_empty() {
            self.filtered_processes.clone()
        } else {
            self.metrics.all_processes()
        };

        // Group by TGID (thread-group / main PID). Aggregate CPU and memory.
        let mut grouped: std::collections::HashMap<u32, SystemProcessInfo> = std::collections::HashMap::new();

        for p in base_list {
            let key = p.tgid;
            grouped
                .entry(key)
                .and_modify(|agg| {
                    agg.cpu_usage += p.cpu_usage;
                    agg.memory_kb = agg.memory_kb.saturating_add(p.memory_kb);
                })
                .or_insert_with(|| SystemProcessInfo {
                    pid: key,   // represent the group by its TGID
                    tgid: key,
                    name: p.name.clone(),
                    cmd: p.cmd.clone(),
                    cpu_usage: p.cpu_usage,
                    memory_kb: p.memory_kb,
                    user: p.user.clone(),
                });
        }

        let grouped_vec: Vec<SystemProcessInfo> = grouped.into_values().collect();
        self.sorted_processes(grouped_vec)
    }

    /// Total number of processes available for navigation.
    pub fn process_count(&self) -> usize {
        self.current_processes().len()
    }

    /// Ensure the selected index is valid for the current process list.
    pub fn clamp_process_selection(&mut self) {
        let total = self.process_count();
        if total == 0 {
            self.selected_process_index = 0;
        } else if self.selected_process_index >= total {
            self.selected_process_index = total - 1;
        }
    }

    /// Apply the current sort mode to a process collection.
    fn sorted_processes(&self, mut processes: Vec<SystemProcessInfo>) -> Vec<SystemProcessInfo> {
        match self.process_sort {
            ProcessSortMode::Cpu => {
                processes.sort_by(|a, b| b.cpu_usage.partial_cmp(&a.cpu_usage).unwrap_or(std::cmp::Ordering::Equal));
            }
            ProcessSortMode::Memory => {
                processes.sort_by(|a, b| b.memory_kb.cmp(&a.memory_kb));
            }
            ProcessSortMode::Name => {
                processes.sort_by(|a, b| a.name.cmp(&b.name));
            }
            ProcessSortMode::Pid => {
                processes.sort_by(|a, b| a.pid.cmp(&b.pid));
            }
        }
        processes
    }

    pub fn show_kill_dialog(&mut self) {
        self.dialog_mode = DialogMode::ConfirmKill;
    }

    pub fn show_terminate_dialog(&mut self) {
        self.dialog_mode = DialogMode::ConfirmTerminate;
    }

    pub fn show_info_dialog(&mut self) {
        self.dialog_mode = DialogMode::ProcessInfo;
    }

    pub fn close_dialog(&mut self) {
        self.dialog_mode = DialogMode::None;
    }

    pub fn kill_selected_process(&mut self) -> Result<()> {
        if let Some(pid) = self.get_selected_pid() {
            log::info!("Force killing process with PID: {}", pid);
            match force_kill(pid) {
                Ok(msg) => {
                    self.set_status_message(msg);
                }
                Err(e) => {
                    self.set_status_message(format!("Failed to kill process {}: {}", pid, e));
                }
            }
        }
        self.close_dialog();
        Ok(())
    }

    pub fn terminate_selected_process(&mut self) -> Result<()> {
        if let Some(pid) = self.get_selected_pid() {
            log::info!("Terminating process with PID: {} (graceful)", pid);
            match crate::process::signals::terminate(pid) {
                Ok(msg) => {
                    self.set_status_message(msg);
                }
                Err(e) => {
                    self.set_status_message(format!("Failed to terminate process {}: {}", pid, e));
                }
            }
        }
        self.close_dialog();
        Ok(())
    }
    
    /// Smart kill with escalation (SIGTERM -> SIGKILL) - async version
    pub async fn smart_kill_selected_process(&mut self) -> Result<()> {
        if let Some(pid) = self.get_selected_pid() {
            log::info!("Smart killing process with PID: {}", pid);
            self.set_status_message(format!("Sending SIGTERM to process {}, waiting...", pid));
            
            match smart_kill(pid).await {
                Ok(msg) => {
                    self.set_status_message(msg);
                }
                Err(e) => {
                    self.set_status_message(format!("Failed to kill process {}: {}", pid, e));
                }
            }
        }
        self.close_dialog();
        Ok(())
    }
    
    /// Toggle tree view mode
    pub fn toggle_tree_mode(&mut self) {
        self.tree_mode = !self.tree_mode;
        if self.tree_mode {
            self.rebuild_tree();
            self.set_status_message("Tree view enabled".to_string());
        } else {
            self.set_status_message("Tree view disabled".to_string());
        }
    }
    
    /// Rebuild process tree
    pub fn rebuild_tree(&mut self) {
        let all_processes = self.metrics.all_processes();
        self.process_tree.build_from_processes(all_processes);
        self.process_tree.calculate_render_order(&self.collapsed_pids);
    }
    
    /// Toggle collapse/expand of selected process in tree
    pub fn toggle_collapse_selected(&mut self) {
        if !self.tree_mode {
            return;
        }
        
        if let Some(pid) = self.get_selected_pid() {
            if self.process_tree.has_children(pid) {
                if self.collapsed_pids.contains(&pid) {
                    self.collapsed_pids.remove(&pid);
                    self.set_status_message(format!("Expanded process {}", pid));
                } else {
                    self.collapsed_pids.insert(pid);
                    self.set_status_message(format!("Collapsed process {}", pid));
                }
                self.process_tree.calculate_render_order(&self.collapsed_pids);
            }
        }
    }
    
    /// Enter filter mode
    pub fn enter_filter_mode(&mut self) {
        self.filter_mode = true;
        self.filter_input.clear();
        self.set_status_message("Filter: (type to search, ESC to cancel)".to_string());
    }
    
    /// Exit filter mode
    pub fn exit_filter_mode(&mut self) {
        self.filter_mode = false;
        self.filter_input.clear();
        self.filtered_processes.clear();
        self.set_status_message("Filter cleared".to_string());
    }
    
    /// Add character to filter input
    pub fn filter_input_char(&mut self, c: char) {
        self.filter_input.push(c);
        self.apply_filter();
    }
    
    /// Remove last character from filter input
    pub fn filter_backspace(&mut self) {
        self.filter_input.pop();
        self.apply_filter();
    }
    
    /// Apply current filter to process list
    pub fn apply_filter(&mut self) {
        let mut processes = if self.filter_input.is_empty() {
            self.metrics.all_processes()
        } else {
            let filter_lower = self.filter_input.to_lowercase();
            self.metrics
                .all_processes()
                .into_iter()
                .filter(|p| {
                    p.name.to_lowercase().contains(&filter_lower)
                        || p.cmd.to_lowercase().contains(&filter_lower)
                        || p.pid.to_string().contains(&filter_lower)
                })
                .collect()
        };

        processes = self.sorted_processes(processes);
        self.filtered_processes = processes;
        self.clamp_process_selection();
    }
    
    /// Set status message with timestamp
    pub fn set_status_message(&mut self, msg: String) {
        log::info!("Status: {}", msg);
        self.status_message = Some(msg);
        self.status_message_time = Some(SystemTime::now());
    }
    
    /// Clear status message if older than 5 seconds
    pub fn update_status_message(&mut self) {
        if let Some(time) = self.status_message_time {
            if let Ok(elapsed) = time.elapsed() {
                if elapsed.as_secs() > 5 {
                    self.status_message = None;
                    self.status_message_time = None;
                }
            }
        }
    }
}
