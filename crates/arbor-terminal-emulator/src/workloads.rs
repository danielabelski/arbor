pub fn prompt_redraw_workload() -> Vec<Vec<u8>> {
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

pub fn wide_scroll_workload() -> Vec<Vec<u8>> {
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

pub fn resume_scroll_workload() -> Vec<Vec<u8>> {
    let mut chunks = Vec::new();

    chunks.push(b"\x1b[?1049h\x1b[2J\x1b[H\x1b[?25l".to_vec());
    chunks.push(b"Select a session to resume\r\n".to_vec());
    chunks.push(b"\r\n".to_vec());
    for row in 1..=18 {
        let marker = if row == 7 {
            ">"
        } else {
            " "
        };
        chunks.push(
            format!(
                "{marker} session-{row:03}  model=gpt-5.4  cwd=~/code/arbor  updated={}s ago\r\n",
                4 * row
            )
            .into_bytes(),
        );
    }

    // Resume redraws often clear the alternate screen before the first
    // transcript frame lands. Keep this as a standalone chunk so GUI tests can
    // assert that Arbor does not flash a blank terminal in between.
    chunks.push(b"\x1b[H\x1b[2J".to_vec());
    chunks.push(b"Resuming session-007\r\n".to_vec());
    chunks.push(b"restoring transcript...\r\n".to_vec());
    chunks.push(b"reconnecting tools...\r\n".to_vec());
    chunks.push(b"\r\n".to_vec());

    for line in 0..260 {
        let role = match line % 4 {
            0 => "system",
            1 => "user",
            2 => "assistant",
            _ => "tool",
        };
        chunks.push(
            format!(
                "{role:>9} {line:03}: resume transcript line {line:03} cwd=~/code/arbor tokens={:05}\r\n",
                10_000 + line * 17
            )
            .into_bytes(),
        );
    }

    chunks.push(b"\x1b[?25h".to_vec());
    chunks
}
