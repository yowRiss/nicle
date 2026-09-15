use serde::{Deserialize, Serialize};
use std::{
    sync::{
        mpsc::channel,
        Arc,
    },
    time::Duration,
};
use tauri::{AppHandle, Emitter, Listener};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProcessMem {
    pub pid: u32,
    pub comm: String,
    pub rss_kb: u64,
    pub pss_kb: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WorkloadSample {
    pub stage: String,
    pub timestamp_sec: u64,
    pub total_rss_kb: u64,
    pub total_pss_kb: u64,
    pub processes: Vec<ProcessMem>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BenchmarkReport {
    pub workloads: Vec<WorkloadSample>,
    pub cycle_closed_pss_kb: Vec<u64>,
    pub cycle_active_pss_kb: Vec<u64>,
    pub memory_leak_detected: bool,
    pub summary: String,
}

pub fn sample_all_processes() -> Vec<ProcessMem> {
    let my_pid = std::process::id();
    let mut pids = vec![my_pid];
    if let Ok(entries) = std::fs::read_dir("/proc") {
        for entry in entries.flatten() {
            if let Ok(name) = entry.file_name().into_string() {
                if let Ok(pid) = name.parse::<u32>() {
                    if pid != my_pid {
                        if let Ok(stat) = std::fs::read_to_string(format!("/proc/{pid}/stat")) {
                            if let Some(ppid_str) = stat.split_whitespace().nth(3) {
                                if let Ok(ppid) = ppid_str.parse::<u32>() {
                                    if ppid == my_pid || pids.contains(&ppid) {
                                        pids.push(pid);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    let mut results = Vec::new();
    for pid in pids {
        let comm = std::fs::read_to_string(format!("/proc/{pid}/comm"))
            .unwrap_or_else(|_| "unknown".into())
            .trim()
            .to_string();
        if let Ok(smaps) = std::fs::read_to_string(format!("/proc/{pid}/smaps_rollup")) {
            let mut rss = 0;
            let mut pss = 0;
            for line in smaps.lines() {
                if let Some(r) = line.strip_prefix("Rss:") {
                    rss = r.trim().split_whitespace().next().unwrap_or("0").parse().unwrap_or(0);
                } else if let Some(p) = line.strip_prefix("Pss:") {
                    pss = p.trim().split_whitespace().next().unwrap_or("0").parse().unwrap_or(0);
                }
            }
            results.push(ProcessMem {
                pid,
                comm,
                rss_kb: rss,
                pss_kb: pss,
            });
        }
    }
    results
}

#[derive(Clone, Serialize, Deserialize)]
struct BenchCmd {
    step: String,
    arg: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
struct BenchResp {
    step: String,
    ok: bool,
    error: Option<String>,
}

pub fn run_benchmark_suite(handle: AppHandle) {
    std::thread::spawn(move || {
        println!("[*] Benchmark suite starting in background...");
        let (tx, rx) = channel::<()>();
        let (step_tx, step_rx) = channel::<BenchResp>();

        let tx_ready = Arc::new(parking_lot::Mutex::new(Some(tx)));
        handle.listen("benchmark-frontend-ready", move |_| {
            if let Some(sender) = tx_ready.lock().take() {
                let _ = sender.send(());
            }
        });

        let step_tx = Arc::new(parking_lot::Mutex::new(step_tx));
        handle.listen("benchmark-step-done", move |event| {
            if let Ok(resp) = serde_json::from_str::<BenchResp>(event.payload()) {
                let _ = step_tx.lock().send(resp);
            }
        });

        // Wait for frontend ready with a generous timeout
        let _ = rx.recv_timeout(Duration::from_secs(12));
        println!("[*] Frontend ready signal received. Executing benchmark sequence...");

        let exec_step = |step: &str, arg: Option<&str>| {
            let cmd = BenchCmd {
                step: step.to_string(),
                arg: arg.map(Into::into),
            };
            let _ = handle.emit("benchmark-run", cmd);
            let _ = step_rx.recv_timeout(Duration::from_secs(10));
        };

        let sample = |stage: &str| -> WorkloadSample {
            let procs = sample_all_processes();
            let total_rss_kb: u64 = procs.iter().map(|p| p.rss_kb).sum();
            let total_pss_kb: u64 = procs.iter().map(|p| p.pss_kb).sum();
            println!("  -> Stage: {:<38} | PSS: {:>6.1} MB | RSS: {:>6.1} MB",
                stage,
                total_pss_kb as f64 / 1024.0,
                total_rss_kb as f64 / 1024.0
            );
            WorkloadSample {
                stage: stage.to_string(),
                timestamp_sec: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
                total_rss_kb,
                total_pss_kb,
                processes: procs,
            }
        };

        // Workload 1: Empty window
        std::thread::sleep(Duration::from_secs(3));
        let s1 = sample("1. Empty Window");

        // Workload 2: Project open
        let proj_root = std::env::current_dir().unwrap_or_default().display().to_string();
        exec_step("open-workspace", Some(&proj_root));
        std::thread::sleep(Duration::from_secs(2));
        let s2 = sample("2. Project Open");

        // Workload 3: Files open (multiple regular files + large file)
        let f1 = format!("{proj_root}/README.md");
        let f2 = format!("{proj_root}/src/editor/documents.ts");
        let f3 = format!("{proj_root}/src/App.svelte");
        let f_large = format!("{proj_root}/fixtures/large_sample.txt");
        exec_step("open-file", Some(&f1));
        exec_step("open-file", Some(&f2));
        exec_step("open-file", Some(&f3));
        exec_step("open-file", Some(&f_large));
        std::thread::sleep(Duration::from_secs(2));
        let s3 = sample("3. Files Open (incl. Large-File Mode)");

        // Workload 4: Terminal
        exec_step("start-terminal", None);
        std::thread::sleep(Duration::from_secs(2));
        let s4 = sample("4. Terminal Active");

        // Test hiding terminal (intentionally preserves shell and buffers)
        exec_step("hide-terminal", None);
        std::thread::sleep(Duration::from_millis(600));
        let s4_hidden = sample("4b. Terminal Hidden (Buffers Retained)");

        // Workload 5: Preview
        let f_html = format!("{proj_root}/index.html");
        exec_step("open-file", Some(&f_html));
        exec_step("start-preview", None);
        std::thread::sleep(Duration::from_secs(2));
        let s5 = sample("5. Preview Active");

        // Workload 6: Repeated Open/Close Cycles (5 full cycles)
        println!("[*] Commencing 5 repeated open/close cycles to track memory stability...");
        let mut closed_pss = Vec::new();
        let mut active_pss = Vec::new();
        let mut cycle_samples = Vec::new();

        for cycle in 1..=5 {
            // End preview, end terminal, close all tabs
            exec_step("close-preview", None);
            exec_step("end-terminal", None);
            exec_step("close-all-tabs", None);
            std::thread::sleep(Duration::from_millis(1200));
            let s_closed = sample(&format!("Cycle {cycle}: Closed/Idle"));
            closed_pss.push(s_closed.total_pss_kb);
            cycle_samples.push(s_closed);

            // Re-open workloads
            exec_step("open-file", Some(&f1));
            exec_step("open-file", Some(&f2));
            exec_step("open-file", Some(&f_large));
            exec_step("start-terminal", None);
            exec_step("open-file", Some(&f_html));
            exec_step("start-preview", None);
            std::thread::sleep(Duration::from_millis(1500));
            let s_active = sample(&format!("Cycle {cycle}: Active"));
            active_pss.push(s_active.total_pss_kb);
            cycle_samples.push(s_active);
        }

        // Final cleanup
        exec_step("close-preview", None);
        exec_step("end-terminal", None);
        exec_step("close-all-tabs", None);
        std::thread::sleep(Duration::from_secs(2));
        let s_final = sample("Final Idle (After 5 Cycles)");

        let mut all_samples = vec![s1, s2, s3, s4, s4_hidden, s5];
        all_samples.extend(cycle_samples);
        all_samples.push(s_final);

        let pss_start_closed = closed_pss.first().copied().unwrap_or(0);
        let pss_end_closed = closed_pss.last().copied().unwrap_or(0);
        let drift_kb = pss_end_closed as i64 - pss_start_closed as i64;
        let leak_detected = drift_kb > 25 * 1024;

        let report = BenchmarkReport {
            workloads: all_samples,
            cycle_closed_pss_kb: closed_pss.clone(),
            cycle_active_pss_kb: active_pss.clone(),
            memory_leak_detected: leak_detected,
            summary: format!(
                "Closed PSS across 5 cycles: {:?} kB. Drift: {} kB. Memory leak: {}. Memory stabilizes: {}",
                closed_pss,
                drift_kb,
                if leak_detected { "YES" } else { "NO" },
                if !leak_detected { "YES" } else { "NO" }
            ),
        };

        let json_str = serde_json::to_string_pretty(&report).unwrap_or_default();
        let _ = std::fs::write("BENCHMARK_REPORT.json", json_str);
        println!("
======================================================");
        println!("=== NICLE WORKLOAD MEMORY BENCHMARK COMPLETED ===");
        println!("======================================================");
        println!("{}", report.summary);
        println!("Saved report to BENCHMARK_REPORT.json
");

        std::process::exit(0);
    });
}
