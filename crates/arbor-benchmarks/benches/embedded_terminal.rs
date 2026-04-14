#![allow(clippy::expect_used, clippy::unwrap_used)]

use arbor_terminal_emulator::{
    TerminalEmulator, TerminalEngineKind, prompt_redraw_workload, resume_scroll_workload,
    wide_scroll_workload,
};

fn main() {
    divan::main();
}

#[divan::bench]
fn process_alacritty(bencher: divan::Bencher) {
    let workload = benchmark_workload();
    bencher.bench_local(|| {
        let mut emulator = TerminalEmulator::with_engine(TerminalEngineKind::Alacritty, 40, 120);
        for chunk in &workload {
            emulator.process(chunk);
        }
        divan::black_box(emulator);
    });
}

#[cfg(feature = "ghostty-vt-experimental")]
#[divan::bench]
fn process_ghostty_vt_experimental(bencher: divan::Bencher) {
    let workload = benchmark_workload();
    bencher.bench_local(|| {
        let mut emulator =
            TerminalEmulator::with_engine(TerminalEngineKind::GhosttyVtExperimental, 40, 120);
        for chunk in &workload {
            emulator.process(chunk);
        }
        divan::black_box(emulator);
    });
}

#[divan::bench]
fn snapshot_alacritty(bencher: divan::Bencher) {
    let emulator = populated_emulator(TerminalEngineKind::Alacritty);
    bencher.bench_local(|| {
        let snapshot = emulator.snapshot();
        assert!(snapshot.output.contains("status: done"));
        assert!(!snapshot.styled_lines.is_empty());
        divan::black_box(snapshot);
    });
}

#[cfg(feature = "ghostty-vt-experimental")]
#[divan::bench]
fn snapshot_ghostty_vt_experimental(bencher: divan::Bencher) {
    let emulator = populated_emulator(TerminalEngineKind::GhosttyVtExperimental);
    bencher.bench_local(|| {
        let snapshot = emulator.snapshot();
        assert!(snapshot.output.contains("status: done"));
        assert!(!snapshot.styled_lines.is_empty());
        divan::black_box(snapshot);
    });
}

#[divan::bench]
fn render_ansi_alacritty(bencher: divan::Bencher) {
    let emulator = populated_emulator(TerminalEngineKind::Alacritty);
    bencher.bench_local(|| {
        let rendered = emulator.render_ansi_snapshot(180);
        assert!(rendered.contains("status: done"));
        divan::black_box(rendered);
    });
}

#[cfg(feature = "ghostty-vt-experimental")]
#[divan::bench]
fn render_ansi_ghostty_vt_experimental(bencher: divan::Bencher) {
    let emulator = populated_emulator(TerminalEngineKind::GhosttyVtExperimental);
    bencher.bench_local(|| {
        let rendered = emulator.render_ansi_snapshot(180);
        assert!(rendered.contains("status: done"));
        divan::black_box(rendered);
    });
}

#[divan::bench]
fn full_roundtrip_alacritty(bencher: divan::Bencher) {
    let workload = benchmark_workload();
    bencher.bench_local(|| {
        let mut emulator = TerminalEmulator::with_engine(TerminalEngineKind::Alacritty, 40, 120);
        for chunk in &workload {
            emulator.process(chunk);
        }
        let snapshot = emulator.snapshot();
        let rendered = emulator.render_ansi_snapshot(180);
        assert!(snapshot.output.contains("status: done"));
        assert!(rendered.contains("status: done"));
        divan::black_box((snapshot, rendered));
    });
}

#[divan::bench]
fn prompt_redraw_snapshot_alacritty(bencher: divan::Bencher) {
    let emulator = populated_emulator_with_workload(
        TerminalEngineKind::Alacritty,
        40,
        120,
        prompt_redraw_workload(),
    );
    bencher.bench_local(|| {
        let snapshot = emulator.snapshot();
        assert!(
            snapshot
                .output
                .contains("Would you like to make the following edits?")
        );
        assert!(snapshot.output.contains("don't ask again for these files"));
        divan::black_box(snapshot);
    });
}

#[divan::bench]
fn prompt_redraw_snapshot_tail_alacritty(bencher: divan::Bencher) {
    let emulator = populated_emulator_with_workload(
        TerminalEngineKind::Alacritty,
        40,
        120,
        prompt_redraw_workload(),
    );
    bencher.bench_local(|| {
        let snapshot = emulator.snapshot_tail(180);
        assert!(
            snapshot
                .output
                .contains("Would you like to make the following edits?")
        );
        assert!(!snapshot.styled_lines.is_empty());
        divan::black_box(snapshot);
    });
}

#[divan::bench]
fn prompt_redraw_render_ansi_alacritty(bencher: divan::Bencher) {
    let emulator = populated_emulator_with_workload(
        TerminalEngineKind::Alacritty,
        40,
        120,
        prompt_redraw_workload(),
    );
    bencher.bench_local(|| {
        let rendered = emulator.render_ansi_snapshot(180);
        assert!(rendered.contains("Would you like to make the following edits?"));
        divan::black_box(rendered);
    });
}

#[divan::bench]
fn wide_scroll_snapshot_alacritty(bencher: divan::Bencher) {
    let emulator = populated_emulator_with_workload(
        TerminalEngineKind::Alacritty,
        40,
        120,
        wide_scroll_workload(),
    );
    bencher.bench_local(|| {
        let snapshot = emulator.snapshot();
        assert!(snapshot.output.contains("Filesystem"));
        assert!(snapshot.output.contains("/Volumes/worktree-219"));
        divan::black_box(snapshot);
    });
}

#[divan::bench]
fn resume_scroll_snapshot_alacritty(bencher: divan::Bencher) {
    let emulator = populated_emulator_with_workload(
        TerminalEngineKind::Alacritty,
        40,
        120,
        resume_scroll_workload(),
    );
    bencher.bench_local(|| {
        let snapshot = emulator.snapshot();
        assert!(snapshot.output.contains("resume transcript line 259"));
        divan::black_box(snapshot);
    });
}

#[divan::bench]
fn resume_scroll_snapshot_tail_alacritty(bencher: divan::Bencher) {
    let emulator = populated_emulator_with_workload(
        TerminalEngineKind::Alacritty,
        40,
        120,
        resume_scroll_workload(),
    );
    bencher.bench_local(|| {
        let snapshot = emulator.snapshot_tail(180);
        assert!(snapshot.output.contains("resume transcript line 259"));
        divan::black_box(snapshot);
    });
}

#[divan::bench]
fn resume_scroll_render_ansi_alacritty(bencher: divan::Bencher) {
    let emulator = populated_emulator_with_workload(
        TerminalEngineKind::Alacritty,
        40,
        120,
        resume_scroll_workload(),
    );
    bencher.bench_local(|| {
        let rendered = emulator.render_ansi_snapshot(180);
        assert!(rendered.contains("resume transcript line 259"));
        divan::black_box(rendered);
    });
}

#[divan::bench]
fn wide_scroll_snapshot_tail_alacritty(bencher: divan::Bencher) {
    let emulator = populated_emulator_with_workload(
        TerminalEngineKind::Alacritty,
        40,
        120,
        wide_scroll_workload(),
    );
    bencher.bench_local(|| {
        let snapshot = emulator.snapshot_tail(180);
        assert!(snapshot.output.contains("Filesystem"));
        assert!(snapshot.output.contains("/Volumes/worktree-219"));
        divan::black_box(snapshot);
    });
}

#[divan::bench]
fn wide_scroll_render_ansi_alacritty(bencher: divan::Bencher) {
    let emulator = populated_emulator_with_workload(
        TerminalEngineKind::Alacritty,
        40,
        120,
        wide_scroll_workload(),
    );
    bencher.bench_local(|| {
        let rendered = emulator.render_ansi_snapshot(180);
        assert!(rendered.contains("Filesystem"));
        assert!(rendered.contains("/Volumes/worktree-219"));
        divan::black_box(rendered);
    });
}

#[cfg(feature = "ghostty-vt-experimental")]
#[divan::bench]
fn full_roundtrip_ghostty_vt_experimental(bencher: divan::Bencher) {
    let workload = benchmark_workload();
    bencher.bench_local(|| {
        let mut emulator =
            TerminalEmulator::with_engine(TerminalEngineKind::GhosttyVtExperimental, 40, 120);
        for chunk in &workload {
            emulator.process(chunk);
        }
        let snapshot = emulator.snapshot();
        let rendered = emulator.render_ansi_snapshot(180);
        assert!(snapshot.output.contains("status: done"));
        assert!(rendered.contains("status: done"));
        divan::black_box((snapshot, rendered));
    });
}

#[cfg(feature = "ghostty-vt-experimental")]
#[divan::bench]
fn prompt_redraw_snapshot_ghostty_vt_experimental(bencher: divan::Bencher) {
    let emulator = populated_emulator_with_workload(
        TerminalEngineKind::GhosttyVtExperimental,
        40,
        120,
        prompt_redraw_workload(),
    );
    bencher.bench_local(|| {
        let snapshot = emulator.snapshot();
        assert!(
            snapshot
                .output
                .contains("Would you like to make the following edits?")
        );
        assert!(snapshot.output.contains("don't ask again for these files"));
        divan::black_box(snapshot);
    });
}

#[cfg(feature = "ghostty-vt-experimental")]
#[divan::bench]
fn prompt_redraw_snapshot_tail_ghostty_vt_experimental(bencher: divan::Bencher) {
    let emulator = populated_emulator_with_workload(
        TerminalEngineKind::GhosttyVtExperimental,
        40,
        120,
        prompt_redraw_workload(),
    );
    bencher.bench_local(|| {
        let snapshot = emulator.snapshot_tail(180);
        assert!(
            snapshot
                .output
                .contains("Would you like to make the following edits?")
        );
        assert!(!snapshot.styled_lines.is_empty());
        divan::black_box(snapshot);
    });
}

#[cfg(feature = "ghostty-vt-experimental")]
#[divan::bench]
fn prompt_redraw_render_ansi_ghostty_vt_experimental(bencher: divan::Bencher) {
    let emulator = populated_emulator_with_workload(
        TerminalEngineKind::GhosttyVtExperimental,
        40,
        120,
        prompt_redraw_workload(),
    );
    bencher.bench_local(|| {
        let rendered = emulator.render_ansi_snapshot(180);
        assert!(rendered.contains("Would you like to make the following edits?"));
        divan::black_box(rendered);
    });
}

#[cfg(feature = "ghostty-vt-experimental")]
#[divan::bench]
fn resume_scroll_snapshot_ghostty_vt_experimental(bencher: divan::Bencher) {
    let emulator = populated_emulator_with_workload(
        TerminalEngineKind::GhosttyVtExperimental,
        40,
        120,
        resume_scroll_workload(),
    );
    bencher.bench_local(|| {
        let snapshot = emulator.snapshot();
        assert!(snapshot.output.contains("resume transcript line 259"));
        divan::black_box(snapshot);
    });
}

#[cfg(feature = "ghostty-vt-experimental")]
#[divan::bench]
fn resume_scroll_snapshot_tail_ghostty_vt_experimental(bencher: divan::Bencher) {
    let emulator = populated_emulator_with_workload(
        TerminalEngineKind::GhosttyVtExperimental,
        40,
        120,
        resume_scroll_workload(),
    );
    bencher.bench_local(|| {
        let snapshot = emulator.snapshot_tail(180);
        assert!(snapshot.output.contains("resume transcript line 259"));
        divan::black_box(snapshot);
    });
}

#[cfg(feature = "ghostty-vt-experimental")]
#[divan::bench]
fn resume_scroll_render_ansi_ghostty_vt_experimental(bencher: divan::Bencher) {
    let emulator = populated_emulator_with_workload(
        TerminalEngineKind::GhosttyVtExperimental,
        40,
        120,
        resume_scroll_workload(),
    );
    bencher.bench_local(|| {
        let rendered = emulator.render_ansi_snapshot(180);
        assert!(rendered.contains("resume transcript line 259"));
        divan::black_box(rendered);
    });
}

#[cfg(feature = "ghostty-vt-experimental")]
#[divan::bench]
fn wide_scroll_snapshot_ghostty_vt_experimental(bencher: divan::Bencher) {
    let emulator = populated_emulator_with_workload(
        TerminalEngineKind::GhosttyVtExperimental,
        40,
        120,
        wide_scroll_workload(),
    );
    bencher.bench_local(|| {
        let snapshot = emulator.snapshot();
        assert!(snapshot.output.contains("Filesystem"));
        assert!(snapshot.output.contains("/Volumes/worktree-219"));
        divan::black_box(snapshot);
    });
}

#[cfg(feature = "ghostty-vt-experimental")]
#[divan::bench]
fn wide_scroll_snapshot_tail_ghostty_vt_experimental(bencher: divan::Bencher) {
    let emulator = populated_emulator_with_workload(
        TerminalEngineKind::GhosttyVtExperimental,
        40,
        120,
        wide_scroll_workload(),
    );
    bencher.bench_local(|| {
        let snapshot = emulator.snapshot_tail(180);
        assert!(snapshot.output.contains("Filesystem"));
        assert!(snapshot.output.contains("/Volumes/worktree-219"));
        divan::black_box(snapshot);
    });
}

#[cfg(feature = "ghostty-vt-experimental")]
#[divan::bench]
fn wide_scroll_render_ansi_ghostty_vt_experimental(bencher: divan::Bencher) {
    let emulator = populated_emulator_with_workload(
        TerminalEngineKind::GhosttyVtExperimental,
        40,
        120,
        wide_scroll_workload(),
    );
    bencher.bench_local(|| {
        let rendered = emulator.render_ansi_snapshot(180);
        assert!(rendered.contains("Filesystem"));
        assert!(rendered.contains("/Volumes/worktree-219"));
        divan::black_box(rendered);
    });
}

fn populated_emulator(engine: TerminalEngineKind) -> TerminalEmulator {
    populated_emulator_with_workload(engine, 40, 120, benchmark_workload())
}

fn populated_emulator_with_workload(
    engine: TerminalEngineKind,
    rows: u16,
    cols: u16,
    workload: Vec<Vec<u8>>,
) -> TerminalEmulator {
    let mut emulator = TerminalEmulator::with_engine(engine, rows, cols);
    for chunk in workload {
        emulator.process(&chunk);
    }
    emulator
}

fn benchmark_workload() -> Vec<Vec<u8>> {
    let mut chunks = Vec::new();

    for frame in 0..200 {
        chunks.push(
            format!(
                "\x1b[38;2;90;180;255mframe {frame:03}\x1b[0m \
                 \x1b[48;2;30;30;30mstatus: running\x1b[0m\r\n"
            )
            .into_bytes(),
        );
    }

    chunks.push(b"\x1b[?1049h\x1b[2J\x1b[H".to_vec());
    for step in 0..120 {
        chunks.push(
            format!(
                "\x1b[{line};1H\x1b[38;5;{color}mstep {step:03} unicode: \u{2603}\u{fe0f}\x1b[0m",
                line = (step % 30) + 1,
                color = 16 + (step % 200),
            )
            .into_bytes(),
        );
    }
    chunks.push(b"\x1b[?1049l".to_vec());

    for row in 0..300 {
        chunks.push(
            format!(
                "log-{row:03} :: \x1b[1mhighlight\x1b[0m :: \x1b[4;38;5;214mwarning\x1b[0m\r\n"
            )
            .into_bytes(),
        );
    }

    chunks.push(b"\x1b]1337;CurrentDir=/tmp\x07".to_vec());
    chunks.push(b"\x1b[?25l".to_vec());
    chunks.push(b"\x1b[?25h".to_vec());
    chunks.push(b"\x1b[?1h".to_vec());
    chunks.push(b"\x1b[?1l".to_vec());
    chunks.push(b"\x1b[38;2;120;255;120mstatus: done\x1b[0m\r\n".to_vec());

    chunks
}
