//! Read-only command-line run of every scanner, for debugging without the UI.
//!
//!     cargo run --release -p mrclean-core --example scan -- [cleaner|files|security|memory|all]

use mrclean_core::fsutil::Progress;
use mrclean_core::sysinfo::System;
use mrclean_core::{bigfiles, cleaner, memory, security, system, Cancel, Env};

fn gb(b: u64) -> String {
    format!("{:.2} GB", b as f64 / 1e9)
}

fn main() {
    let what = std::env::args().nth(1).unwrap_or_else(|| "all".into());
    let env = Env::detect();
    let cancel = Cancel::new();
    let t = std::time::Instant::now();

    let mut sys = System::new_all();
    sys.refresh_all();
    let info = system::info(&sys);
    println!("{} · {} · {} cores · RAM {}", info.os_version, info.cpu_brand, info.cpu_cores, gb(info.memory.total_bytes));
    if let Some(d) = &info.disk {
        println!("Disk {}: {} free of {}", d.mount, gb(d.free_bytes), gb(d.total_bytes));
    }

    if what == "cleaner" || what == "all" {
        let s = cleaner::scan(&env, &cancel, &Progress::default());
        println!("\n== Cleaner: {} found, {} safe ({:?})", gb(s.total_bytes), gb(s.safe_bytes), t.elapsed());
        for r in &s.rules {
            println!("  {:<40} {:>10}  {:?}  {} item(s)", r.rule.name, gb(r.total_bytes), r.rule.safety, r.items.len());
        }
        let nm = cleaner::node_modules::find(&env, std::slice::from_ref(&env.home), 90, &cancel);
        println!("  node_modules folders: {}", nm.len());
    }
    if what == "files" || what == "all" {
        let t = std::time::Instant::now();
        let s = bigfiles::scan(&env.home, &cancel, &Progress::default()).summary();
        println!("\n== Files: {} in {} files, {} unreadable ({:?})", gb(s.total_bytes), s.file_count, s.unreadable, t.elapsed());
        for f in s.top_files.iter().take(5) {
            println!("  {:>10}  {}", gb(f.bytes), f.path);
        }
    }
    if what == "security" || what == "all" {
        let t = std::time::Instant::now();
        let mut roots = vec![env.home.clone()];
        if let Ok(extra) = std::env::var("MRCLEAN_ROOTS") {
            roots.extend(extra.split(':').map(std::path::PathBuf::from));
        }
        let r = security::scan(&env, &roots, &cancel);
        println!("\n== Security: {:?} ({} repos, {} packages, {:?})", r.counts, r.scanned_repos, r.scanned_packages, t.elapsed());
        for f in &r.findings {
            println!("  [{:?}] {} — {}", f.severity, f.title, f.path.as_deref().unwrap_or(""));
        }
    }
    if what == "memory" || what == "all" {
        memory::refresh(&mut sys);
        let s = memory::snapshot(&sys);
        println!("\n== Memory: top apps");
        for a in s.apps.iter().take(5) {
            println!("  {:<30} {:>10} protected={}", a.app, gb(a.memory_bytes), a.protected);
        }
        for p in &s.dev_leftovers {
            println!("  dev: {:?} {} {}", p.dev_kind, p.name, p.command.chars().take(90).collect::<String>());
        }
    }
}
