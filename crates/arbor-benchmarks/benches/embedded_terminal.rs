#![allow(clippy::expect_used, clippy::unwrap_used)]

use arbor_terminal_emulator::{TerminalEmulator, TerminalEngineKind};

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

fn prompt_redraw_workload() -> Vec<Vec<u8>> {
    let mut chunks = Vec::new();

    for frame in 0..120 {
        chunks.push(b"\x1b[H\x1b[2J".to_vec());
        chunks.push(b"  Would you like to make the following edits?\r\n".to_vec());
        chunks.push(b"\r\n".to_vec());
        chunks.push(b"  crates/arbor-gui/src/app_init.rs (+4 -0)\r\n".to_vec());
        chunks.push(
            b"    223  -        self.terminal_scroll_handle: ScrollHandle::new(),\r\n".to_vec(),
        );
        chunks.push(b"    224  +        terminal_follow_output_until: None,\r\n".to_vec());
        chunks.push(b"    225  +        last_terminal_scroll_offset_y: None,\r\n".to_vec());
        chunks.push(b"\r\n".to_vec());
        chunks.push(b"  1. Yes, proceed (y)\r\n".to_vec());
        chunks.push(b"  2. Yes, and don't ask again for these files (a)\r\n".to_vec());
        chunks.push(b"  3. No, and tell Codex what to do differently (esc)".to_vec());
        chunks.push(b"\x1b[3A".to_vec());
        match frame % 3 {
            0 => {
                chunks.push(b"\r\x1b[2K\xe2\x80\xba 1. Yes, proceed (y)\n".to_vec());
                chunks
                    .push(b"\r\x1b[2K  2. Yes, and don't ask again for these files (a)\n".to_vec());
                chunks.push(
                    b"\r\x1b[2K  3. No, and tell Codex what to do differently (esc)\n".to_vec(),
                );
            },
            1 => {
                chunks.push(b"\r\x1b[2K  1. Yes, proceed (y)\n".to_vec());
                chunks.push(
                    b"\r\x1b[2K\xe2\x80\xba 2. Yes, and don't ask again for these files (a)\n"
                        .to_vec(),
                );
                chunks.push(
                    b"\r\x1b[2K  3. No, and tell Codex what to do differently (esc)\n".to_vec(),
                );
            },
            _ => {
                chunks.push(b"\r\x1b[2K  1. Yes, proceed (y)\n".to_vec());
                chunks
                    .push(b"\r\x1b[2K  2. Yes, and don't ask again for these files (a)\n".to_vec());
                chunks.push(
                    b"\r\x1b[2K\xe2\x80\xba 3. No, and tell Codex what to do differently (esc)\n"
                        .to_vec(),
                );
            },
        }
    }

    chunks
}

fn wide_scroll_workload() -> Vec<Vec<u8>> {
    let mut chunks = Vec::new();
    chunks.push(b"\x1b[H\x1b[2J".to_vec());
    chunks.push(b"Filesystem             Size   Used  Avail Capacity Mounted on\r\n".to_vec());

    for row in 0..220 {
        let used_gib = (row * 7) % 900 + 50;
        let avail_gib = 1024 - used_gib;
        let capacity = (used_gib * 100) / 1024;
        chunks.push(
            format!(
                "/dev/disk{row:<3}         1.0Ti  {used_gib:>4}Gi  {avail_gib:>4}Gi    {capacity:>2}%   /Volumes/worktree-{row:03}\r\n"
            )
            .into_bytes(),
        );
    }

    chunks
}
