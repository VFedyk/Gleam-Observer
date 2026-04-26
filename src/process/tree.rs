use crate::metrics::system::ProcessInfo;
use std::collections::{HashMap, HashSet};
use std::fs;

#[derive(Clone, Debug)]
pub struct ProcessNode {
    pub pid: u32,
    pub ppid: u32,
    pub info: ProcessInfo,
    pub children: Vec<u32>,
}

pub struct ProcessTree {
    nodes: HashMap<u32, ProcessNode>,
    #[allow(dead_code)]
    root_pid: u32,
    render_order: Vec<(u32, usize)>, // (pid, depth)
}

impl ProcessTree {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            root_pid: 1,
            render_order: Vec::new(),
        }
    }

    /// Build tree from flat process list
    pub fn build_from_processes(&mut self, processes: Vec<ProcessInfo>) {
        self.nodes.clear();
        
        // First pass: create all nodes
        for proc in processes {
            let ppid = Self::read_ppid(proc.pid).unwrap_or(0);
            let node = ProcessNode {
                pid: proc.pid,
                ppid,
                info: proc,
                children: Vec::new(),
            };
            self.nodes.insert(node.pid, node);
        }

        // Second pass: build parent-child relationships
        let pids: Vec<u32> = self.nodes.keys().copied().collect();
        for pid in pids {
            if let Some(node) = self.nodes.get(&pid) {
                let ppid = node.ppid;
                if ppid != 0 && ppid != pid {
                    if let Some(parent) = self.nodes.get_mut(&ppid) {
                        parent.children.push(pid);
                    }
                }
            }
        }

        // Sort children by PID for consistent ordering
        for node in self.nodes.values_mut() {
            node.children.sort();
        }
    }

    /// Read parent PID from /proc/[pid]/stat
    fn read_ppid(pid: u32) -> Option<u32> {
        let stat_path = format!("/proc/{}/stat", pid);
        let content = fs::read_to_string(stat_path).ok()?;
        
        // Format: pid (comm) state ppid ...
        // Find the closing parenthesis of comm (process name can contain spaces/parens)
        let rparen_idx = content.rfind(')')?;
        let after_comm = &content[rparen_idx + 1..];
        let parts: Vec<&str> = after_comm.split_whitespace().collect();
        
        // ppid is the second field after comm (state is first)
        if parts.len() >= 2 {
            parts[1].parse::<u32>().ok()
        } else {
            None
        }
    }

    /// Generate render order with depth information
    pub fn calculate_render_order(&mut self, collapsed_pids: &HashSet<u32>) {
        self.render_order.clear();
        
        // Find root processes (orphaned or direct children of init)
        let mut roots: Vec<u32> = self.nodes
            .iter()
            .filter(|(_pid, node)| {
                node.ppid == 0 || 
                node.ppid == 1 || 
                !self.nodes.contains_key(&node.ppid)
            })
            .map(|(pid, _)| *pid)
            .collect();
        
        roots.sort();

        for root_pid in roots {
            self.dfs_render(root_pid, 0, collapsed_pids);
        }
    }

    fn dfs_render(&mut self, pid: u32, depth: usize, collapsed_pids: &HashSet<u32>) {
        self.render_order.push((pid, depth));

        // If this node is collapsed, don't recurse into children
        if collapsed_pids.contains(&pid) {
            return;
        }

        if let Some(node) = self.nodes.get(&pid) {
            let children = node.children.clone();
            for child_pid in children {
                self.dfs_render(child_pid, depth + 1, collapsed_pids);
            }
        }
    }

    /// Get processes in render order (respects tree hierarchy and collapsed nodes)
    pub fn iter_visible(&self) -> impl Iterator<Item = (&ProcessNode, usize)> + '_ {
        self.render_order.iter().filter_map(|(pid, depth)| {
            self.nodes.get(pid).map(|node| (node, *depth))
        })
    }

    /// Get a specific node
    pub fn get_node(&self, pid: u32) -> Option<&ProcessNode> {
        self.nodes.get(&pid)
    }

    /// Check if process has children
    pub fn has_children(&self, pid: u32) -> bool {
        self.nodes.get(&pid).map(|n| !n.children.is_empty()).unwrap_or(false)
    }

    /// Get total number of processes in tree
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Check if tree is empty
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Get aggregated CPU usage for a subtree (node + all descendants)
    pub fn get_aggregated_cpu(&self, pid: u32) -> f32 {
        let mut total = 0.0;
        if let Some(node) = self.nodes.get(&pid) {
            total += node.info.cpu_usage;
            for &child_pid in &node.children {
                total += self.get_aggregated_cpu(child_pid);
            }
        }
        total
    }

    /// Get aggregated memory usage for a subtree
    pub fn get_aggregated_memory(&self, pid: u32) -> u64 {
        let mut total = 0u64;
        if let Some(node) = self.nodes.get(&pid) {
            total += node.info.memory_kb;
            for &child_pid in &node.children {
                total += self.get_aggregated_memory(child_pid);
            }
        }
        total
    }
}

impl Default for ProcessTree {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tree_building() {
        let mut tree = ProcessTree::new();
        let processes = vec![
            ProcessInfo {
                pid: 1,
                tgid: 1,
                name: "init".to_string(),
                cmd: "init".to_string(),
                cpu_usage: 0.0,
                memory_kb: 1000,
                user: "root".to_string(),
            },
            ProcessInfo {
                pid: 100,
                tgid: 100,
                name: "parent".to_string(),
                cmd: "parent".to_string(),
                cpu_usage: 10.0,
                memory_kb: 2000,
                user: "root".to_string(),
            },
            ProcessInfo {
                pid: 200,
                tgid: 200,
                name: "child".to_string(),
                cmd: "child".to_string(),
                cpu_usage: 5.0,
                memory_kb: 1500,
                user: "root".to_string(),
            },
        ];

        tree.build_from_processes(processes);
        assert_eq!(tree.len(), 3);
    }

    #[test]
    fn test_collapsed_nodes() {
        let mut tree = ProcessTree::new();
        let mut collapsed = HashSet::new();
        collapsed.insert(100);

        tree.calculate_render_order(&collapsed);
        // If parent (100) is collapsed, its children shouldn't appear in render order
    }
}
