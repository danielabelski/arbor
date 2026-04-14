use {
    arbor_terminal_emulator::{
        TerminalEmulator, TerminalEngineKind, prompt_redraw_workload, resume_scroll_workload,
        wide_scroll_workload,
    },
    std::time::{Duration, Instant},
};

#[derive(Debug, Clone, Copy)]
struct BenchmarkResult {
    process: Duration,
    snapshot: Duration,
    snapshot_tail: Duration,
    render_ansi: Duration,
}

#[derive(Debug, Clone, Copy)]
enum WorkloadKind {
    PromptRedraw,
    ResumeScroll,
    WideScroll,
}

impl WorkloadKind {
    fn label(self) -> &'static str {
        match self {
            Self::PromptRedraw => "prompt-redraw",
            Self::ResumeScroll => "resume-scroll",
            Self::WideScroll => "wide-scroll",
        }
    }
}

#[test]
#[ignore = "benchmark helper; run with -- --ignored --nocapture"]
fn benchmark_terminal_workloads() {
    let iterations = 40;
    let rows = 40;
    let cols = 120;
    let tail_lines = 180;

    println!("terminal workload benchmark ({iterations} iterations)");
    println!(
        "{:<18} {:>12} {:>12} {:>17} {:>14} {:>12}",
        "workload", "process_ms", "snapshot_ms", "snapshot_tail_ms", "render_ms", "total_ms"
    );

    for workload in [
        WorkloadKind::PromptRedraw,
        WorkloadKind::ResumeScroll,
        WorkloadKind::WideScroll,
    ] {
        let result = benchmark_workload(workload, iterations, rows, cols, tail_lines);
        print_result(workload.label(), result);
    }
}

fn benchmark_workload(
    workload: WorkloadKind,
    iterations: usize,
    rows: u16,
    cols: u16,
    tail_lines: usize,
) -> BenchmarkResult {
    let chunks = workload_chunks(workload);
    let mut process = Duration::ZERO;
    let mut snapshot = Duration::ZERO;
    let mut snapshot_tail = Duration::ZERO;
    let mut render_ansi = Duration::ZERO;

    for _ in 0..iterations {
        let mut emulator = TerminalEmulator::with_engine(TerminalEngineKind::Alacritty, rows, cols);

        let process_started = Instant::now();
        for chunk in &chunks {
            emulator.process(chunk);
        }
        process += process_started.elapsed();

        let snapshot_started = Instant::now();
        let terminal_snapshot = emulator.snapshot();
        snapshot += snapshot_started.elapsed();
        assert_snapshot(
            workload,
            &terminal_snapshot.output,
            &terminal_snapshot.styled_lines,
        );

        let snapshot_tail_started = Instant::now();
        let tail_snapshot = emulator.snapshot_tail(tail_lines);
        snapshot_tail += snapshot_tail_started.elapsed();
        assert_snapshot(workload, &tail_snapshot.output, &tail_snapshot.styled_lines);

        let render_started = Instant::now();
        let rendered = emulator.render_ansi_snapshot(tail_lines);
        render_ansi += render_started.elapsed();
        assert_rendered(workload, &rendered);
    }

    BenchmarkResult {
        process,
        snapshot,
        snapshot_tail,
        render_ansi,
    }
}

fn assert_snapshot(
    workload: WorkloadKind,
    output: &str,
    styled_lines: &[arbor_terminal_emulator::TerminalStyledLine],
) {
    assert!(
        !styled_lines.is_empty(),
        "missing styled lines for {}",
        workload.label()
    );

    match workload {
        WorkloadKind::PromptRedraw => {
            assert!(
                output.contains("Would you like to make the following edits?"),
                "missing prompt header in {} output",
                workload.label()
            );
            assert!(
                output.contains("don't ask again for these files"),
                "missing prompt selection text in {} output",
                workload.label()
            );
        },
        WorkloadKind::ResumeScroll => {
            assert!(
                output.contains("resume transcript line 259"),
                "missing final resume line in {} output",
                workload.label()
            );
        },
        WorkloadKind::WideScroll => {
            assert!(
                output.contains("/Volumes/worktree-219"),
                "missing final df-like row in {} output",
                workload.label()
            );
        },
    }
}

fn assert_rendered(workload: WorkloadKind, rendered: &str) {
    match workload {
        WorkloadKind::PromptRedraw => {
            assert!(
                rendered.contains("Would you like to make the following edits?"),
                "missing prompt header in rendered {} output",
                workload.label()
            );
        },
        WorkloadKind::ResumeScroll => {
            assert!(
                rendered.contains("resume transcript line 259"),
                "missing final resume line in rendered {} output",
                workload.label()
            );
        },
        WorkloadKind::WideScroll => {
            assert!(
                rendered.contains("/Volumes/worktree-219"),
                "missing final df-like row in rendered {} output",
                workload.label()
            );
        },
    }
}

fn print_result(name: &str, result: BenchmarkResult) {
    let process_ms = result.process.as_secs_f64() * 1000.0;
    let snapshot_ms = result.snapshot.as_secs_f64() * 1000.0;
    let snapshot_tail_ms = result.snapshot_tail.as_secs_f64() * 1000.0;
    let render_ms = result.render_ansi.as_secs_f64() * 1000.0;
    let total_ms = process_ms + snapshot_ms + snapshot_tail_ms + render_ms;
    println!(
        "{:<18} {:>12.2} {:>12.2} {:>17.2} {:>14.2} {:>12.2}",
        name, process_ms, snapshot_ms, snapshot_tail_ms, render_ms, total_ms
    );
}

fn workload_chunks(workload: WorkloadKind) -> Vec<Vec<u8>> {
    match workload {
        WorkloadKind::PromptRedraw => prompt_redraw_workload(),
        WorkloadKind::ResumeScroll => resume_scroll_workload(),
        WorkloadKind::WideScroll => wide_scroll_workload(),
    }
}
