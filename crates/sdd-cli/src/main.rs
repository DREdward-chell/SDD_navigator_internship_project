use clap::{Parser, Subcommand};
use sdd_core::{
    compute_coverage, parse_requirements, parse_tasks, scan_directory, CoverageStatus, TaskStatus,
};
use std::path::{Path, PathBuf};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

// ---------------------------------------------------------------------------
// CLI definition
// ---------------------------------------------------------------------------

/// @req SCS-CLI-001
/// @req SCS-SELF-001
#[derive(Parser)]
#[command(name = "sdd-coverage", about = "SDD coverage scanner")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Scan a project for @req annotation coverage
    Scan {
        /// Path to requirements.yaml
        #[arg(long, default_value = "requirements.yaml")]
        requirements: PathBuf,

        /// Path to tasks.yaml
        #[arg(long, default_value = "tasks.yaml")]
        tasks: PathBuf,

        /// Root directory to scan for source files
        #[arg(long, default_value = ".")]
        source: PathBuf,

        /// Strict mode: exit 1 if any requirement is not fully covered or any orphan exists
        #[arg(long)]
        strict: bool,
    },
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

fn main() {
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn")))
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Scan {
            requirements,
            tasks,
            source,
            strict,
        } => {
            run_scan(&requirements, &tasks, &source, strict);
        }
    }
}

// ---------------------------------------------------------------------------
// Scan execution
// ---------------------------------------------------------------------------

/// @req SCS-CLI-001
fn run_scan(requirements_path: &Path, tasks_path: &Path, source_path: &Path, strict: bool) {
    let reqs = match parse_requirements(requirements_path) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Error reading requirements: {e}");
            std::process::exit(2);
        }
    };

    let tasks = match parse_tasks(tasks_path) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Error reading tasks: {e}");
            std::process::exit(2);
        }
    };

    let annotations = match scan_directory(source_path) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("Error scanning source: {e}");
            std::process::exit(2);
        }
    };

    let result = compute_coverage(&reqs, &tasks, &annotations);
    let stats = &result.stats;

    // ----- Summary table -----
    println!("SDD Coverage Report");
    println!("{}", "=".repeat(50));
    println!();
    println!("Requirements: {}", stats.requirements.total);
    println!(
        "  covered:  {:>4}",
        stats
            .requirements
            .by_status
            .get("covered")
            .copied()
            .unwrap_or(0)
    );
    println!(
        "  partial:  {:>4}",
        stats
            .requirements
            .by_status
            .get("partial")
            .copied()
            .unwrap_or(0)
    );
    println!(
        "  missing:  {:>4}",
        stats
            .requirements
            .by_status
            .get("missing")
            .copied()
            .unwrap_or(0)
    );
    println!();
    println!("Annotations: {}", stats.annotations.total);
    println!("  impl:     {:>4}", stats.annotations.impl_count);
    println!("  test:     {:>4}", stats.annotations.test);
    println!("  orphans:  {:>4}", stats.annotations.orphans);
    println!();
    println!("Tasks: {}", stats.tasks.total);
    println!(
        "  done:         {:>4}",
        stats.tasks.by_status.get("done").copied().unwrap_or(0)
    );
    println!(
        "  in_progress:  {:>4}",
        stats
            .tasks
            .by_status
            .get("in_progress")
            .copied()
            .unwrap_or(0)
    );
    println!(
        "  open:         {:>4}",
        stats.tasks.by_status.get("open").copied().unwrap_or(0)
    );
    println!("  orphans:      {:>4}", stats.tasks.orphans);
    println!();
    println!("Coverage: {:.2}%", stats.coverage);
    println!("{}", "=".repeat(50));

    if !strict {
        std::process::exit(0);
    }

    // ----- Strict mode evaluation -----
    let partial: Vec<_> = result
        .requirements
        .iter()
        .filter(|r| r.status == CoverageStatus::Partial)
        .collect();
    let missing: Vec<_> = result
        .requirements
        .iter()
        .filter(|r| r.status == CoverageStatus::Missing)
        .collect();
    let orphan_annotations = &result.orphan_annotations;
    let orphan_tasks: Vec<_> = result
        .orphan_tasks
        .iter()
        .filter(|t| t.status != TaskStatus::Done)
        .collect();

    let has_failures = !partial.is_empty()
        || !missing.is_empty()
        || !orphan_annotations.is_empty()
        || !orphan_tasks.is_empty();

    if has_failures {
        println!();
        println!("STRICT MODE: FAILED");

        if !partial.is_empty() {
            println!("  Partial requirements ({}):", partial.len());
            for r in &partial {
                println!("    {}: {}", r.id, r.title);
            }
        }
        if !missing.is_empty() {
            println!("  Missing requirements ({}):", missing.len());
            for r in &missing {
                println!("    {}: {}", r.id, r.title);
            }
        }
        if !orphan_annotations.is_empty() {
            println!("  Orphan annotations ({}):", orphan_annotations.len());
            for a in orphan_annotations {
                println!("    {} ({}:{})", a.req_id, a.file, a.line);
            }
        }
        if !orphan_tasks.is_empty() {
            println!("  Orphan tasks ({}):", orphan_tasks.len());
            for t in &orphan_tasks {
                println!("    {}: {}", t.id, t.title);
            }
        }
        std::process::exit(1);
    }

    println!();
    println!("STRICT MODE: PASSED");
    std::process::exit(0);
}
