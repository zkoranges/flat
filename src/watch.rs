use anyhow::Result;
use std::time::{Duration, Instant};
use notify::{Watcher, RecursiveMode, recommended_watcher};
use std::sync::mpsc;

use crate::Config;
use crate::walker::walk_and_flatten;

/// Enter watch mode: monitor for file changes and regenerate output
pub fn watch_mode(config: Config) -> Result<()> {
    eprintln!("🔍 Watch mode enabled. Monitoring {} for changes...", config.path.display());
    eprintln!("   Press Ctrl+C to exit.");
    eprintln!();

    // Setup file watcher with channel for events
    let (tx, rx) = mpsc::channel();
    let mut watcher = recommended_watcher(tx)
        .map_err(|e| anyhow::anyhow!("Failed to create watcher: {}", e))?;
    watcher.watch(&config.path, RecursiveMode::Recursive)
        .map_err(|e| anyhow::anyhow!("Failed to watch path: {}", e))?;

    // Setup Ctrl+C handler
    let should_exit = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let exit_clone = should_exit.clone();
    ctrlc::set_handler(move || {
        eprintln!("\n📍 Shutting down watch mode...");
        exit_clone.store(true, std::sync::atomic::Ordering::SeqCst);
    })?;

    // Initial run
    eprintln!("📝 Running initial flattening...");
    if let Err(e) = walk_and_flatten(&config) {
        eprintln!("❌ Error during initial run: {}", e);
    } else {
        eprintln!("✅ Initial flattening complete.\n");
    }

    // Event loop with debouncing
    let mut last_event_time = Instant::now();
    let mut pending_rebuild = false;
    let debounce_duration = Duration::from_millis(500);

    loop {
        if should_exit.load(std::sync::atomic::Ordering::SeqCst) {
            break;
        }

        // Wait for events with timeout
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(_event) => {
                // Got a file change event
                last_event_time = Instant::now();
                pending_rebuild = true;
                eprintln!("📌 File change detected");
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                // Check if we should rebuild (debounce window settled)
                if pending_rebuild
                    && last_event_time.elapsed() >= debounce_duration
                {
                    eprintln!("🔄 Debounce settled, rebuilding...");
                    if let Err(e) = walk_and_flatten(&config) {
                        eprintln!("❌ Error during rebuild: {}", e);
                    } else {
                        eprintln!("✅ Rebuild complete.\n");
                    }
                    pending_rebuild = false;
                }
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                eprintln!("⚠️  File watcher disconnected");
                break;
            }
        }
    }

    eprintln!("👋 Watch mode exited.");
    Ok(())
}