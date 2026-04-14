use {
    super::*,
    std::{
        collections::hash_map::DefaultHasher,
        hash::{Hash, Hasher},
    },
};

const TERMINAL_RENDER_OVERSCAN_LINES: usize = 12;
const TERMINAL_INITIAL_RENDER_LINES: usize = 240;

pub(crate) struct TerminalRenderSource<'a> {
    pub(crate) session_id: u64,
    pub(crate) state: TerminalState,
    pub(crate) output: &'a str,
    pub(crate) styled_output: &'a [TerminalStyledLine],
    pub(crate) cursor: Option<TerminalCursor>,
}

pub(crate) fn terminal_render_source_for_session(
    session: &TerminalSession,
) -> TerminalRenderSource<'_> {
    TerminalRenderSource {
        session_id: session.id,
        state: session.state,
        output: &session.output,
        styled_output: &session.styled_output,
        cursor: session.cursor,
    }
}

pub(crate) fn terminal_render_source_for_snapshot<'a>(
    session_id: u64,
    state: TerminalState,
    snapshot: &'a arbor_terminal_emulator::TerminalSnapshot,
) -> TerminalRenderSource<'a> {
    TerminalRenderSource {
        session_id,
        state,
        output: &snapshot.output,
        styled_output: &snapshot.styled_lines,
        cursor: snapshot.cursor,
    }
}

pub(crate) fn terminal_styled_lines_have_visible_content(lines: &[TerminalStyledLine]) -> bool {
    lines
        .iter()
        .any(|line| !(line.cells.is_empty() && line.runs.is_empty()))
}

pub(crate) fn terminal_styled_line_has_non_whitespace_text(line: &TerminalStyledLine) -> bool {
    if !line.cells.is_empty() {
        return line
            .cells
            .iter()
            .flat_map(|cell| cell.text.chars())
            .any(|character| !character.is_whitespace());
    }

    line.runs
        .iter()
        .flat_map(|run| run.text.chars())
        .any(|character| !character.is_whitespace())
}

pub(crate) fn terminal_styled_lines_have_non_whitespace_text(lines: &[TerminalStyledLine]) -> bool {
    lines
        .iter()
        .any(terminal_styled_line_has_non_whitespace_text)
}

pub(crate) fn terminal_last_non_whitespace_line_index(
    lines: &[TerminalStyledLine],
) -> Option<usize> {
    lines
        .iter()
        .rposition(terminal_styled_line_has_non_whitespace_text)
}

pub(crate) fn terminal_excerpt(text: &str, max_chars: usize) -> String {
    let mut output = String::new();
    let mut truncated = false;

    for (index, character) in text.chars().enumerate() {
        if index >= max_chars {
            truncated = true;
            break;
        }
        output.extend(character.escape_debug());
    }

    if truncated {
        output.push_str("...");
    }

    output
}

pub(crate) fn terminal_styled_line_excerpt(line: &TerminalStyledLine, max_chars: usize) -> String {
    terminal_excerpt(&styled_line_to_string(line), max_chars)
}

pub(crate) fn terminal_render_source_has_visible_content(
    source: &TerminalRenderSource<'_>,
) -> bool {
    if !source.styled_output.is_empty() {
        return terminal_styled_lines_have_visible_content(source.styled_output);
    }

    !source.output.is_empty()
}

#[cfg(test)]
pub(crate) fn styled_lines_for_session(
    session: &TerminalSession,
    theme: ThemePalette,
    show_cursor: bool,
    selection: Option<&TerminalSelection>,
    ime_marked_text: Option<&str>,
) -> Vec<TerminalStyledLine> {
    let source = terminal_render_source_for_session(session);
    let line_count = terminal_render_line_count_for_source(&source, selection);
    styled_lines_for_render_source_range(
        &source,
        theme,
        show_cursor,
        selection,
        ime_marked_text,
        0..line_count,
    )
}

pub(crate) fn styled_lines_for_session_range(
    session: &TerminalSession,
    theme: ThemePalette,
    show_cursor: bool,
    selection: Option<&TerminalSelection>,
    ime_marked_text: Option<&str>,
    range: std::ops::Range<usize>,
) -> Vec<TerminalStyledLine> {
    let source = terminal_render_source_for_session(session);
    styled_lines_for_render_source_range(
        &source,
        theme,
        show_cursor,
        selection,
        ime_marked_text,
        range,
    )
}

pub(crate) fn styled_lines_for_render_source_range(
    source: &TerminalRenderSource<'_>,
    theme: ThemePalette,
    show_cursor: bool,
    selection: Option<&TerminalSelection>,
    ime_marked_text: Option<&str>,
    range: std::ops::Range<usize>,
) -> Vec<TerminalStyledLine> {
    if range.is_empty() {
        return Vec::new();
    }

    let mut lines = if !source.styled_output.is_empty() {
        let start = range.start.min(source.styled_output.len());
        let end = range.end.min(source.styled_output.len());
        source.styled_output[start..end].to_vec()
    } else {
        plain_lines_to_styled(
            lines_for_display(source.output, false)
                .into_iter()
                .skip(range.start)
                .take(range.len())
                .collect(),
            theme,
        )
    };

    remap_terminal_line_palette(&mut lines, theme);

    if show_cursor
        && source.state == TerminalState::Running
        && let Some(cursor) = source.cursor
        && range.contains(&cursor.line)
    {
        let cursor = TerminalCursor {
            line: cursor.line - range.start,
            column: cursor.column,
        };
        if let Some(marked) = ime_marked_text {
            apply_ime_marked_text_to_lines(&mut lines, cursor, marked, theme);
        } else {
            apply_cursor_to_lines(&mut lines, cursor, theme);
        }
    }

    if let Some(selection) = selection.filter(|selection| selection.session_id == source.session_id)
        && let Some(selection) = terminal_selection_for_render_range(selection, &range)
    {
        apply_selection_to_lines(&mut lines, &selection, theme);
    }

    lines
}

pub(crate) fn terminal_render_line_count(
    session: &TerminalSession,
    selection: Option<&TerminalSelection>,
) -> usize {
    terminal_render_line_count_for_source(&terminal_render_source_for_session(session), selection)
}

pub(crate) fn terminal_render_line_count_for_source(
    source: &TerminalRenderSource<'_>,
    selection: Option<&TerminalSelection>,
) -> usize {
    let base_count = if !source.styled_output.is_empty() {
        source
            .styled_output
            .iter()
            .rposition(terminal_styled_line_has_non_whitespace_text)
            .map(|index| index + 1)
            .unwrap_or(0)
    } else {
        lines_for_display(source.output, false)
            .iter()
            .rposition(|line| line.chars().any(|character| !character.is_whitespace()))
            .map(|index| index + 1)
            .unwrap_or(0)
    };

    let cursor_count = source
        .cursor
        .map_or(0, |cursor| cursor.line.saturating_add(1));
    let selection_count = selection
        .and_then(normalized_terminal_selection)
        .map_or(0, |(_, end)| end.line.saturating_add(1));

    base_count.max(1).max(cursor_count).max(selection_count)
}

pub(crate) fn terminal_visible_line_range(
    scroll_handle: &ScrollHandle,
    line_count: usize,
    line_height: f32,
) -> std::ops::Range<usize> {
    let line_count = line_count.max(1);
    let viewport_height = scroll_handle.bounds().size.height.to_f64() as f32;

    if !viewport_height.is_finite()
        || viewport_height <= 0.
        || !line_height.is_finite()
        || line_height <= 0.
    {
        let end = line_count;
        let start = end.saturating_sub(TERMINAL_INITIAL_RENDER_LINES);
        return start..end;
    }

    let scroll_top = (-(scroll_handle.offset().y.to_f64() as f32)).max(0.);
    let first_visible_line = (scroll_top / line_height).floor().max(0.) as usize;
    let visible_line_count = (viewport_height / line_height).ceil().max(1.) as usize;
    let start = first_visible_line.saturating_sub(TERMINAL_RENDER_OVERSCAN_LINES);
    let end = line_count.min(
        first_visible_line
            .saturating_add(visible_line_count)
            .saturating_add(TERMINAL_RENDER_OVERSCAN_LINES),
    );

    let start = start.min(line_count.saturating_sub(1));
    let end = end.max(start.saturating_add(1)).min(line_count);
    start..end
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct TerminalRenderSliceLayout {
    pub(crate) content_height_px: f32,
    pub(crate) slice_offset_px: f32,
    pub(crate) slice_height_px: f32,
}

pub(crate) fn terminal_render_slice_layout(
    total_line_count: usize,
    first_visible_line: usize,
    rendered_line_count: usize,
    line_height_px: f32,
) -> TerminalRenderSliceLayout {
    let total_line_count = total_line_count.max(1);
    let rendered_line_count = rendered_line_count.max(1);
    let line_height_px = if line_height_px.is_finite() && line_height_px > 0. {
        line_height_px
    } else {
        1.
    };

    TerminalRenderSliceLayout {
        content_height_px: total_line_count as f32 * line_height_px,
        slice_offset_px: first_visible_line as f32 * line_height_px,
        slice_height_px: rendered_line_count as f32 * line_height_px,
    }
}

pub(crate) fn terminal_viewport_slice_range(
    slice_start_line: usize,
    rendered_line_count: usize,
    scroll_top_px: f32,
    viewport_height_px: f32,
    line_height_px: f32,
) -> std::ops::Range<usize> {
    let rendered_line_count = rendered_line_count.max(1);
    let line_height_px = if line_height_px.is_finite() && line_height_px > 0. {
        line_height_px
    } else {
        1.
    };
    let scroll_top_px = if scroll_top_px.is_finite() && scroll_top_px > 0. {
        scroll_top_px
    } else {
        0.
    };
    let viewport_height_px = if viewport_height_px.is_finite() && viewport_height_px > 0. {
        viewport_height_px
    } else {
        line_height_px
    };

    let first_visible_line = (scroll_top_px / line_height_px).floor().max(0.) as usize;
    let visible_line_count = (viewport_height_px / line_height_px).ceil().max(1.) as usize;
    let start = first_visible_line
        .saturating_sub(slice_start_line)
        .min(rendered_line_count.saturating_sub(1));
    let end = start
        .saturating_add(visible_line_count)
        .max(start.saturating_add(1))
        .min(rendered_line_count);
    start..end
}

pub(crate) fn terminal_visible_render_signature_for_source(
    source: &TerminalRenderSource<'_>,
    visible_range: std::ops::Range<usize>,
) -> u64 {
    let mut hasher = DefaultHasher::new();
    source.state.hash(&mut hasher);
    source.cursor.hash(&mut hasher);
    visible_range.start.hash(&mut hasher);
    visible_range.end.hash(&mut hasher);
    terminal_render_line_count_for_source(source, None).hash(&mut hasher);

    if !source.styled_output.is_empty() {
        let start = visible_range.start.min(source.styled_output.len());
        let end = visible_range.end.min(source.styled_output.len());
        source.styled_output[start..end].hash(&mut hasher);
    } else {
        for line in lines_for_display(source.output, false)
            .into_iter()
            .skip(visible_range.start)
            .take(visible_range.len())
        {
            line.hash(&mut hasher);
        }
    }

    hasher.finish()
}

fn remap_terminal_line_palette(lines: &mut [TerminalStyledLine], theme: ThemePalette) {
    for line in lines {
        remap_terminal_styled_line_palette(line, theme);
    }
}

fn remap_terminal_styled_line_palette(line: &mut TerminalStyledLine, theme: ThemePalette) {
    if line.cells.is_empty() && !line.runs.is_empty() {
        line.cells = cells_from_runs(&line.runs);
    } else if line.runs.is_empty() && !line.cells.is_empty() {
        line.runs = runs_from_cells(&line.cells);
    }

    for cell in &mut line.cells {
        remap_terminal_colors(&mut cell.fg, &mut cell.bg, theme);
    }

    for run in &mut line.runs {
        remap_terminal_colors(&mut run.fg, &mut run.bg, theme);
    }
}

fn remap_terminal_colors(fg: &mut u32, bg: &mut u32, theme: ThemePalette) {
    if *bg == EMBEDDED_TERMINAL_DEFAULT_BG {
        *bg = theme.terminal_bg;
    }
    if *fg == EMBEDDED_TERMINAL_DEFAULT_FG {
        *fg = theme.text_primary;
    }
}

fn terminal_selection_for_render_range(
    selection: &TerminalSelection,
    range: &std::ops::Range<usize>,
) -> Option<TerminalSelection> {
    let (start, end) = normalized_terminal_selection(selection)?;
    if end.line < range.start || start.line >= range.end {
        return None;
    }

    let clamped_start_line = start.line.max(range.start);
    let clamped_end_line = end.line.min(range.end.saturating_sub(1));

    Some(TerminalSelection {
        session_id: selection.session_id,
        anchor: TerminalGridPosition {
            line: clamped_start_line - range.start,
            column: if start.line < range.start {
                0
            } else {
                start.column
            },
        },
        head: TerminalGridPosition {
            line: clamped_end_line - range.start,
            column: if end.line >= range.end {
                usize::MAX
            } else {
                end.column
            },
        },
    })
}

pub(crate) fn apply_cursor_to_lines(
    lines: &mut Vec<TerminalStyledLine>,
    cursor: TerminalCursor,
    theme: ThemePalette,
) {
    while lines.len() <= cursor.line {
        lines.push(TerminalStyledLine {
            cells: Vec::new(),
            runs: Vec::new(),
        });
    }

    if let Some(line) = lines.get_mut(cursor.line) {
        if line.cells.is_empty() && !line.runs.is_empty() {
            line.cells = cells_from_runs(&line.runs);
        }

        let insert_index = line
            .cells
            .iter()
            .position(|cell| cell.column >= cursor.column)
            .unwrap_or(line.cells.len());

        if line
            .cells
            .get(insert_index)
            .is_none_or(|cell| cell.column != cursor.column)
        {
            line.cells.insert(insert_index, TerminalStyledCell {
                column: cursor.column,
                text: " ".to_owned(),
                fg: theme.text_primary,
                bg: theme.terminal_bg,
            });
        }

        if let Some(cell) = line.cells.get_mut(insert_index) {
            if cell.text.is_empty() {
                cell.text = " ".to_owned();
            }

            if cell.text.chars().all(|character| character == ' ') {
                cell.fg = theme.text_primary;
            }
            cell.bg = theme.terminal_cursor;
        }

        line.runs = runs_from_cells(&line.cells);
    }
}

pub(crate) fn apply_ime_marked_text_to_lines(
    lines: &mut [TerminalStyledLine],
    cursor: TerminalCursor,
    marked_text: &str,
    theme: ThemePalette,
) {
    if lines.len() <= cursor.line {
        return;
    }

    let Some(line) = lines.get_mut(cursor.line) else {
        return;
    };

    if line.cells.is_empty() && !line.runs.is_empty() {
        line.cells = cells_from_runs(&line.runs);
    }

    let insert_index = line
        .cells
        .iter()
        .position(|cell| cell.column >= cursor.column)
        .unwrap_or(line.cells.len());

    // Insert marked text cell at cursor position with cursor highlight
    if line
        .cells
        .get(insert_index)
        .is_some_and(|cell| cell.column == cursor.column)
    {
        line.cells[insert_index] = TerminalStyledCell {
            column: cursor.column,
            text: marked_text.to_owned(),
            fg: theme.text_primary,
            bg: theme.terminal_cursor,
        };
    } else {
        line.cells.insert(insert_index, TerminalStyledCell {
            column: cursor.column,
            text: marked_text.to_owned(),
            fg: theme.text_primary,
            bg: theme.terminal_cursor,
        });
    }

    line.runs = runs_from_cells(&line.cells);
}

pub(crate) fn apply_selection_to_lines(
    lines: &mut Vec<TerminalStyledLine>,
    selection: &TerminalSelection,
    theme: ThemePalette,
) {
    let Some((start, end)) = normalized_terminal_selection(selection) else {
        return;
    };

    while lines.len() <= end.line {
        lines.push(TerminalStyledLine {
            cells: Vec::new(),
            runs: Vec::new(),
        });
    }

    for line_index in start.line..=end.line {
        let Some(line) = lines.get_mut(line_index) else {
            continue;
        };
        if line.cells.is_empty() && !line.runs.is_empty() {
            line.cells = cells_from_runs(&line.runs);
        }

        let line_start = if line_index == start.line {
            start.column
        } else {
            0
        };
        let line_end_exclusive = if line_index == end.line {
            end.column
        } else {
            usize::MAX
        };
        if line_end_exclusive <= line_start {
            continue;
        }

        let mut changed = false;
        for cell in &mut line.cells {
            if cell.column >= line_start && cell.column < line_end_exclusive {
                cell.fg = theme.terminal_selection_fg;
                cell.bg = theme.terminal_selection_bg;
                changed = true;
            }
        }

        if changed {
            line.runs = runs_from_cells(&line.cells);
        }
    }
}

pub(crate) fn normalized_terminal_selection(
    selection: &TerminalSelection,
) -> Option<(TerminalGridPosition, TerminalGridPosition)> {
    let (start, end) = if selection.anchor.line < selection.head.line
        || (selection.anchor.line == selection.head.line
            && selection.anchor.column <= selection.head.column)
    {
        (selection.anchor, selection.head)
    } else {
        (selection.head, selection.anchor)
    };

    if start == end {
        return None;
    }

    Some((start, end))
}

pub(crate) fn cells_from_runs(runs: &[TerminalStyledRun]) -> Vec<TerminalStyledCell> {
    let mut cells = Vec::new();
    let mut column = 0_usize;
    for run in runs {
        for character in run.text.chars() {
            cells.push(TerminalStyledCell {
                column,
                text: character.to_string(),
                fg: run.fg,
                bg: run.bg,
            });
            column = column.saturating_add(1);
        }
    }
    cells
}

pub(crate) fn runs_from_cells(cells: &[TerminalStyledCell]) -> Vec<TerminalStyledRun> {
    let mut runs = Vec::new();
    let mut current_fg = None;
    let mut current_bg = None;
    let mut current_text = String::new();
    let mut next_expected_column: Option<usize> = None;
    let mut current_contains_complex_cell = false;
    let mut current_contains_decorative_cell = false;

    for cell in cells {
        let cell_is_complex = cell.text.chars().count() != 1;
        let cell_is_powerline = cell
            .text
            .chars()
            .next()
            .is_some_and(is_terminal_powerline_character)
            && cell.text.chars().count() == 1;
        let style_changed = current_fg != Some(cell.fg) || current_bg != Some(cell.bg);
        let gap_breaks_run = next_expected_column != Some(cell.column);
        let complex_breaks_run = current_contains_complex_cell || cell_is_complex;
        let decorative_breaks_run = current_contains_decorative_cell || cell_is_powerline;
        if style_changed || gap_breaks_run || complex_breaks_run || decorative_breaks_run {
            if let (Some(fg), Some(bg)) = (current_fg.take(), current_bg.take())
                && !current_text.is_empty()
            {
                runs.push(TerminalStyledRun {
                    text: std::mem::take(&mut current_text),
                    fg,
                    bg,
                });
            }

            current_fg = Some(cell.fg);
            current_bg = Some(cell.bg);
            current_contains_complex_cell = cell_is_complex;
            current_contains_decorative_cell = cell_is_powerline;
        }

        current_text.push_str(&cell.text);
        next_expected_column = Some(cell.column.saturating_add(1));
        current_contains_decorative_cell |= cell_is_powerline;
    }

    if let (Some(fg), Some(bg)) = (current_fg, current_bg)
        && !current_text.is_empty()
    {
        runs.push(TerminalStyledRun {
            text: current_text,
            fg,
            bg,
        });
    }

    runs
}

#[derive(Clone)]
pub(crate) struct PositionedTerminalRun {
    pub(crate) text: String,
    pub(crate) fg: u32,
    pub(crate) bg: u32,
    pub(crate) start_column: usize,
    pub(crate) cell_count: usize,
    pub(crate) force_cell_width: bool,
}

#[derive(Clone)]
struct ShapedTerminalRun {
    shaped_line: gpui::ShapedLine,
    bg: u32,
    start_column: usize,
    cell_count: usize,
    force_cell_width: bool,
}

struct ShapedTerminalLine {
    runs: Vec<ShapedTerminalRun>,
}

pub(crate) fn positioned_runs_from_cells(
    cells: &[TerminalStyledCell],
) -> Vec<PositionedTerminalRun> {
    let mut runs = Vec::new();
    let mut current_fg: Option<u32> = None;
    let mut current_bg: Option<u32> = None;
    let mut current_start_column = 0_usize;
    let mut current_text = String::new();
    let mut next_expected_column: Option<usize> = None;
    let mut current_contains_complex_cell = false;
    let mut current_contains_decorative_cell = false;
    let mut current_cell_count = 0_usize;

    for cell in cells {
        let cell_is_complex = cell.text.chars().count() != 1;
        let cell_is_powerline = cell
            .text
            .chars()
            .next()
            .is_some_and(is_terminal_powerline_character)
            && cell.text.chars().count() == 1;
        let style_changed = current_fg != Some(cell.fg) || current_bg != Some(cell.bg);
        let gap_breaks_run = next_expected_column != Some(cell.column);
        let complex_breaks_run = current_contains_complex_cell || cell_is_complex;
        let decorative_breaks_run = current_contains_decorative_cell || cell_is_powerline;
        if style_changed || gap_breaks_run || complex_breaks_run || decorative_breaks_run {
            if let (Some(fg), Some(bg)) = (current_fg.take(), current_bg.take())
                && !current_text.is_empty()
            {
                runs.push(PositionedTerminalRun {
                    text: std::mem::take(&mut current_text),
                    fg,
                    bg,
                    start_column: current_start_column,
                    cell_count: current_cell_count,
                    force_cell_width: !current_contains_complex_cell
                        && !current_contains_decorative_cell,
                });
            }

            current_fg = Some(cell.fg);
            current_bg = Some(cell.bg);
            current_start_column = cell.column;
            current_contains_complex_cell = cell_is_complex;
            current_contains_decorative_cell = cell_is_powerline;
            current_cell_count = 0;
        }

        current_text.push_str(&cell.text);
        current_cell_count = current_cell_count.saturating_add(1);
        current_contains_complex_cell |= cell_is_complex;
        current_contains_decorative_cell |= cell_is_powerline;
        next_expected_column = Some(cell.column.saturating_add(1));
    }

    if let (Some(fg), Some(bg)) = (current_fg, current_bg)
        && !current_text.is_empty()
    {
        runs.push(PositionedTerminalRun {
            text: current_text,
            fg,
            bg,
            start_column: current_start_column,
            cell_count: current_cell_count,
            force_cell_width: !current_contains_complex_cell && !current_contains_decorative_cell,
        });
    }

    runs
}

pub(crate) fn is_terminal_powerline_character(ch: char) -> bool {
    matches!(ch as u32, 0xE0B0..=0xE0D7)
}

pub(crate) fn plain_lines_to_styled(
    lines: Vec<String>,
    theme: ThemePalette,
) -> Vec<TerminalStyledLine> {
    lines
        .into_iter()
        .map(|line| {
            let cells: Vec<TerminalStyledCell> = line
                .chars()
                .enumerate()
                .map(|(column, character)| TerminalStyledCell {
                    column,
                    text: character.to_string(),
                    fg: theme.text_primary,
                    bg: theme.terminal_bg,
                })
                .collect();

            let runs = if line.is_empty() {
                Vec::new()
            } else {
                vec![TerminalStyledRun {
                    text: line,
                    fg: theme.text_primary,
                    bg: theme.terminal_bg,
                }]
            };

            TerminalStyledLine { cells, runs }
        })
        .collect()
}

pub(crate) fn render_terminal_lines(
    session_id: u64,
    lines: Vec<TerminalStyledLine>,
    theme: ThemePalette,
    cell_width: f32,
    line_height: f32,
    mono_font: gpui::Font,
    total_line_count: usize,
    first_visible_line: usize,
) -> Div {
    let line_height = px(line_height);
    let line_height_px = line_height.to_f64() as f32;
    let font_size = px(TERMINAL_FONT_SIZE_PX);
    let total_line_count = total_line_count.max(1);
    let slice_layout = terminal_render_slice_layout(
        total_line_count,
        first_visible_line,
        lines.len(),
        line_height_px,
    );
    let render_debug_enabled = terminal_snapshot_debug_enabled();

    div()
        .flex_none()
        .w_full()
        .min_w_0()
        .h(px(slice_layout.content_height_px))
        .relative()
        .child(
            div()
                .absolute()
                .left(px(0.))
                .top(px(slice_layout.slice_offset_px))
                .w_full()
                .h(px(slice_layout.slice_height_px))
                .overflow_hidden()
                .bg(rgb(theme.terminal_bg))
                .child(
                    canvas(
                        move |_, window, _| {
                            let shaped_lines = lines
                                .into_iter()
                                .map(|line| {
                                    let cells = if line.cells.is_empty() {
                                        cells_from_runs(&line.runs)
                                    } else {
                                        line.cells
                                    };
                                    let runs = positioned_runs_from_cells(&cells)
                                        .into_iter()
                                        .filter(|run| !run.text.is_empty())
                                        .map(|run| {
                                            let is_powerline = should_force_powerline(&run);
                                            let force_cell_width =
                                                run.force_cell_width || is_powerline;
                                            let force_width = if force_cell_width {
                                                Some(px(cell_width))
                                            } else {
                                                None
                                            };
                                            let shaped_line = window.text_system().shape_line(
                                                run.text.clone().into(),
                                                font_size,
                                                &[TextRun {
                                                    len: run.text.len(),
                                                    font: mono_font.clone(),
                                                    color: rgb(run.fg).into(),
                                                    background_color: None,
                                                    underline: None,
                                                    strikethrough: None,
                                                }],
                                                force_width,
                                            );

                                            ShapedTerminalRun {
                                                shaped_line,
                                                bg: run.bg,
                                                start_column: run.start_column,
                                                cell_count: run.cell_count,
                                                force_cell_width,
                                            }
                                        })
                                        .collect();

                                    ShapedTerminalLine { runs }
                                })
                                .collect::<Vec<_>>();

                            if render_debug_enabled {
                                let shaped_run_count: usize =
                                    shaped_lines.iter().map(|line| line.runs.len()).sum();
                                tracing::info!(
                                    session_id,
                                    first_visible_line,
                                    total_line_count,
                                    slice_offset_px = slice_layout.slice_offset_px,
                                    slice_height_px = slice_layout.slice_height_px,
                                    shaped_line_count = shaped_lines.len(),
                                    shaped_run_count,
                                    "terminal canvas shape trace"
                                );
                            }

                            shaped_lines
                        },
                        move |bounds, shaped_lines, window, cx| {
                            let scale_factor = window.scale_factor();
                            if render_debug_enabled {
                                let shaped_run_count: usize =
                                    shaped_lines.iter().map(|line| line.runs.len()).sum();
                                tracing::info!(
                                    session_id,
                                    first_visible_line,
                                    total_line_count,
                                    slice_offset_px = slice_layout.slice_offset_px,
                                    slice_height_px = slice_layout.slice_height_px,
                                    bounds_width = bounds.size.width.to_f64(),
                                    bounds_height = bounds.size.height.to_f64(),
                                    shaped_line_count = shaped_lines.len(),
                                    shaped_run_count,
                                    "terminal canvas paint trace"
                                );
                            }
                            window.paint_quad(fill(bounds, rgb(theme.terminal_bg)));
                            for (line_index, line) in shaped_lines.iter().enumerate() {
                                let line_y =
                                    bounds.origin.y + px(line_index as f32 * line_height_px);
                                for run in &line.runs {
                                    if run.cell_count > 0 {
                                        let start_x = snap_pixels_floor(
                                            bounds.origin.x
                                                + px(run.start_column as f32 * cell_width),
                                            scale_factor,
                                        );
                                        let end_x = snap_pixels_ceil(
                                            bounds.origin.x
                                                + px((run.start_column + run.cell_count) as f32
                                                    * cell_width),
                                            scale_factor,
                                        );
                                        let background_origin = point(start_x, line_y);
                                        let background_size =
                                            size((end_x - start_x).max(px(0.)), line_height);
                                        window.paint_quad(fill(
                                            Bounds::new(background_origin, background_size),
                                            rgb(run.bg),
                                        ));
                                    }

                                    let run_origin =
                                        bounds.origin.x + px(run.start_column as f32 * cell_width);
                                    let run_x = if run.force_cell_width {
                                        run_origin
                                    } else {
                                        run_origin.floor()
                                    };

                                    let _ = run.shaped_line.paint(
                                        point(run_x, line_y),
                                        line_height,
                                        window,
                                        cx,
                                    );
                                }
                            }
                        },
                    )
                    .size_full(),
                ),
        )
}

pub(crate) fn should_force_powerline(run: &PositionedTerminalRun) -> bool {
    run.text.chars().count() == 1
        && run
            .text
            .chars()
            .next()
            .is_some_and(is_terminal_powerline_character)
}

pub(crate) fn snap_pixels_floor(value: Pixels, scale_factor: f32) -> Pixels {
    if !(scale_factor.is_finite() && scale_factor > 0.) {
        return value.floor();
    }

    let scaled = value.to_f64() as f32 * scale_factor;
    px(scaled.floor() / scale_factor)
}

pub(crate) fn snap_pixels_ceil(value: Pixels, scale_factor: f32) -> Pixels {
    if !(scale_factor.is_finite() && scale_factor > 0.) {
        return value.ceil();
    }

    let scaled = value.to_f64() as f32 * scale_factor;
    px(scaled.ceil() / scale_factor)
}

pub(crate) fn lines_for_display(text: &str, placeholder_when_empty: bool) -> Vec<String> {
    if text.is_empty() && placeholder_when_empty {
        return vec!["<no output yet>".to_owned()];
    }

    if text.is_empty() {
        return vec![String::new()];
    }

    text.lines().map(ToOwned::to_owned).collect()
}

pub(crate) fn terminal_display_lines(session: &TerminalSession) -> Vec<String> {
    terminal_display_lines_for_source(&terminal_render_source_for_session(session))
}

pub(crate) fn terminal_display_lines_for_source(source: &TerminalRenderSource<'_>) -> Vec<String> {
    if !source.styled_output.is_empty() {
        return source
            .styled_output
            .iter()
            .map(styled_line_to_string)
            .collect();
    }

    lines_for_display(source.output, false)
}

pub(crate) fn terminal_display_tail_lines(
    session: &TerminalSession,
    max_lines: usize,
) -> Vec<String> {
    terminal_display_tail_lines_for_source(&terminal_render_source_for_session(session), max_lines)
}

pub(crate) fn terminal_display_tail_lines_for_source(
    source: &TerminalRenderSource<'_>,
    max_lines: usize,
) -> Vec<String> {
    if max_lines == 0 {
        return Vec::new();
    }

    if !source.styled_output.is_empty() {
        let end = source
            .styled_output
            .iter()
            .rposition(|line| !(line.cells.is_empty() && line.runs.is_empty()))
            .map_or(1, |index| index + 1)
            .min(source.styled_output.len());
        let start = end.saturating_sub(max_lines);
        return source.styled_output[start..end]
            .iter()
            .map(styled_line_to_string)
            .collect();
    }

    if source.output.is_empty() {
        return vec![String::new()];
    }

    let mut lines: Vec<String> = source
        .output
        .lines()
        .rev()
        .take(max_lines)
        .map(ToOwned::to_owned)
        .collect();
    lines.reverse();
    lines
}

pub(crate) fn styled_line_to_string(line: &TerminalStyledLine) -> String {
    if line.cells.is_empty() {
        return styled_cells_to_string(cells_from_runs(&line.runs).iter());
    }

    if line
        .cells
        .windows(2)
        .all(|window| window[0].column <= window[1].column)
    {
        return styled_cells_to_string(line.cells.iter());
    }

    let mut cells = line.cells.clone();
    cells.sort_by_key(|cell| cell.column);
    styled_cells_to_string(cells.iter())
}

fn styled_cells_to_string<'a>(cells: impl IntoIterator<Item = &'a TerminalStyledCell>) -> String {
    let mut output = String::new();
    let mut current_column = 0_usize;

    for cell in cells {
        while current_column < cell.column {
            output.push(' ');
            current_column = current_column.saturating_add(1);
        }
        output.push_str(&cell.text);
        current_column = current_column.saturating_add(1);
    }

    output
}

pub(crate) fn terminal_grid_position_from_pointer(
    position: gpui::Point<Pixels>,
    bounds: Bounds<Pixels>,
    scroll_offset: gpui::Point<Pixels>,
    line_height: f32,
    cell_width: f32,
    line_count: usize,
) -> Option<TerminalGridPosition> {
    if line_height <= 0. || cell_width <= 0. || line_count == 0 {
        return None;
    }

    let local_x = f32::from(position.x - bounds.left()).max(0.);
    let local_y = f32::from(position.y - bounds.top()).max(0.);
    let content_y = (local_y - f32::from(scroll_offset.y)).max(0.);

    let max_line = line_count.saturating_sub(1);
    let line = ((content_y / line_height).floor() as usize).min(max_line);
    let column = (local_x / cell_width).floor().max(0.) as usize;

    Some(TerminalGridPosition { line, column })
}

pub(crate) fn terminal_token_bounds(
    lines: &[String],
    point: TerminalGridPosition,
) -> Option<(TerminalGridPosition, TerminalGridPosition)> {
    let line = lines.get(point.line)?;
    let chars: Vec<char> = line.chars().collect();
    if chars.is_empty() {
        return None;
    }

    let index = point.column.min(chars.len().saturating_sub(1));
    if chars
        .get(index)
        .is_none_or(|character| character.is_whitespace())
    {
        return None;
    }

    let mut start = index;
    while start > 0 && !chars[start - 1].is_whitespace() {
        start -= 1;
    }

    let mut end = index.saturating_add(1);
    while end < chars.len() && !chars[end].is_whitespace() {
        end += 1;
    }

    Some((
        TerminalGridPosition {
            line: point.line,
            column: start,
        },
        TerminalGridPosition {
            line: point.line,
            column: end,
        },
    ))
}

pub(crate) fn terminal_line_bounds(
    lines: &[String],
    point: TerminalGridPosition,
) -> Option<(TerminalGridPosition, TerminalGridPosition)> {
    let line = lines.get(point.line)?;
    let width = line.chars().count();
    if width == 0 {
        return None;
    }

    Some((
        TerminalGridPosition {
            line: point.line,
            column: 0,
        },
        TerminalGridPosition {
            line: point.line,
            column: width,
        },
    ))
}

pub(crate) fn terminal_selection_text(lines: &[String], selection: &TerminalSelection) -> String {
    let Some((start, end)) = normalized_terminal_selection(selection) else {
        return String::new();
    };

    let mut output = String::new();
    for line_index in start.line..=end.line {
        let line = lines.get(line_index).map_or("", String::as_str);
        let chars: Vec<char> = line.chars().collect();

        let from = if line_index == start.line {
            start.column.min(chars.len())
        } else {
            0
        };
        let to = if line_index == end.line {
            end.column.min(chars.len())
        } else {
            chars.len()
        };

        if from < to {
            output.extend(chars[from..to].iter());
        }

        if line_index != end.line {
            output.push('\n');
        }
    }

    output
}

pub(crate) fn terminal_scroll_is_near_bottom(scroll_handle: &ScrollHandle) -> bool {
    let max_offset = scroll_handle.max_offset();
    if max_offset.height <= px(0.) {
        return true;
    }

    let offset = scroll_handle.offset();
    let distance_from_bottom = (offset.y + max_offset.height).abs();
    distance_from_bottom <= px(TERMINAL_CELL_HEIGHT_PX)
}

pub(crate) fn terminal_follow_lock_is_active(
    follow_output_until: Option<Instant>,
    now: Instant,
) -> bool {
    follow_output_until.is_some_and(|until| until > now)
}

pub(crate) fn terminal_interactive_follow_is_active(
    interactive_sync_until: Option<Instant>,
    now: Instant,
) -> bool {
    interactive_sync_until.is_some_and(|until| until > now)
}

pub(crate) fn terminal_scroll_moved_away_from_bottom(
    previous_offset_y: Option<Pixels>,
    current_offset_y: Pixels,
    is_near_bottom: bool,
) -> bool {
    !is_near_bottom
        && previous_offset_y.is_some_and(|previous| current_offset_y > previous + px(1.))
}

pub(crate) fn terminal_scroll_extent_changed(
    previous_max_offset_y: Option<Pixels>,
    current_max_offset_y: Pixels,
) -> bool {
    previous_max_offset_y.is_none_or(|previous| (current_max_offset_y - previous).abs() > px(1.))
}

pub(crate) fn terminal_grid_size_from_scroll_handle_with_metrics(
    scroll_handle: &ScrollHandle,
    cell_width: f32,
    line_height: f32,
) -> Option<(u16, u16, u16, u16)> {
    let bounds = scroll_handle.bounds();
    let width = (bounds.size.width.to_f64() as f32 - TERMINAL_SCROLLBAR_WIDTH_PX).max(1.);
    let height = bounds.size.height.to_f64() as f32;
    let (rows, cols) = terminal_grid_size_for_viewport(width, height, cell_width, line_height)?;
    let pixel_width = width.floor().clamp(1., f32::from(u16::MAX)) as u16;
    let pixel_height = height.floor().clamp(1., f32::from(u16::MAX)) as u16;
    Some((rows, cols, pixel_width, pixel_height))
}

pub(crate) fn terminal_cell_width_px(cx: &App) -> f32 {
    let text_system = cx.text_system();
    let mono_font = terminal_mono_font(cx);
    let font_id = text_system.resolve_font(&mono_font);

    text_system
        .advance(font_id, px(TERMINAL_FONT_SIZE_PX), 'm')
        .map(|size| size.width.to_f64() as f32)
        .ok()
        .filter(|width| width.is_finite() && *width > 0.)
        .unwrap_or(TERMINAL_CELL_WIDTH_PX)
}

pub(crate) fn diff_cell_width_px(cx: &App) -> f32 {
    let text_system = cx.text_system();
    let mono_font = terminal_mono_font(cx);
    let font_id = text_system.resolve_font(&mono_font);
    let fallback = (TERMINAL_CELL_WIDTH_PX * (DIFF_FONT_SIZE_PX / TERMINAL_FONT_SIZE_PX)).max(1.);

    text_system
        .advance(font_id, px(DIFF_FONT_SIZE_PX), 'm')
        .map(|size| size.width.to_f64() as f32)
        .ok()
        .filter(|width| width.is_finite() && *width > 0.)
        .unwrap_or(fallback)
}

pub(crate) fn terminal_line_height_px(cx: &App) -> f32 {
    let text_system = cx.text_system();
    let mono_font = terminal_mono_font(cx);
    let font_id = text_system.resolve_font(&mono_font);
    let font_size = px(TERMINAL_FONT_SIZE_PX);

    let ascent = text_system.ascent(font_id, font_size).to_f64() as f32;
    let descent = text_system.descent(font_id, font_size).to_f64() as f32;
    let measured_height = if descent.is_sign_negative() {
        ascent - descent
    } else {
        ascent + descent
    };

    if measured_height.is_finite() && measured_height > 0. {
        return measured_height.ceil().max(TERMINAL_FONT_SIZE_PX).max(1.);
    }

    TERMINAL_CELL_HEIGHT_PX
}

pub(crate) fn terminal_grid_size_for_viewport(
    width: f32,
    height: f32,
    cell_width: f32,
    cell_height: f32,
) -> Option<(u16, u16)> {
    if width <= 0. || height <= 0. || cell_width <= 0. || cell_height <= 0. {
        return None;
    }

    let cols = (width / cell_width).floor() as i32;
    let rows = (height / cell_height).floor() as i32;
    if cols <= 0 || rows <= 0 {
        return None;
    }

    let cols = cols.clamp(2, i32::from(u16::MAX)) as u16;
    let rows = rows.clamp(1, i32::from(u16::MAX)) as u16;
    Some((rows, cols))
}

pub(crate) fn should_auto_follow_terminal_output(
    terminal_updated: bool,
    should_follow_output: bool,
) -> bool {
    terminal_updated && should_follow_output
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TerminalScheduledFollowPassDecision {
    pub(crate) perform_scroll: bool,
    pub(crate) schedule_retry: bool,
}

pub(crate) fn terminal_scheduled_follow_pass_decision(
    pass_index: usize,
    max_passes: usize,
    should_follow: bool,
    should_scroll: bool,
) -> TerminalScheduledFollowPassDecision {
    if !should_follow || max_passes == 0 {
        return TerminalScheduledFollowPassDecision {
            perform_scroll: false,
            schedule_retry: false,
        };
    }

    let has_retry_budget = pass_index.saturating_add(1) < max_passes;
    TerminalScheduledFollowPassDecision {
        perform_scroll: should_scroll,
        schedule_retry: has_retry_budget && (pass_index == 0 || should_scroll),
    }
}

pub(crate) fn terminal_should_follow_output(
    is_near_bottom: bool,
    follow_lock_active: bool,
    interactive_follow_active: bool,
    sticky_follow_active: bool,
) -> bool {
    is_near_bottom || follow_lock_active || interactive_follow_active || sticky_follow_active
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use {
        super::*,
        crate::{daemon_runtime::session_with_styled_line, theme::ThemeKind},
        arbor_terminal_emulator::TerminalEmulator,
    };

    #[test]
    fn cursor_is_painted_at_terminal_column_instead_of_line_end() {
        let theme = ThemeKind::One.palette();
        let session = session_with_styled_line(
            "abcdef",
            0x112233,
            0x445566,
            Some(TerminalCursor { line: 0, column: 2 }),
        );

        let lines = styled_lines_for_session(&session, theme, true, None, None);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].runs.len(), 3);
        assert_eq!(lines[0].runs[0].text, "ab");
        assert_eq!(lines[0].runs[1].text, "c");
        assert_eq!(lines[0].runs[1].fg, 0x112233);
        assert_eq!(lines[0].runs[1].bg, theme.terminal_cursor);
        assert_eq!(lines[0].runs[2].text, "def");
    }

    fn styled_line(text: &str) -> TerminalStyledLine {
        TerminalStyledLine {
            cells: text
                .chars()
                .enumerate()
                .map(|(column, character)| TerminalStyledCell {
                    column,
                    text: character.to_string(),
                    fg: 0x112233,
                    bg: 0x445566,
                })
                .collect(),
            runs: vec![TerminalStyledRun {
                text: text.to_owned(),
                fg: 0x112233,
                bg: 0x445566,
            }],
        }
    }

    #[test]
    fn terminal_render_line_count_trims_trailing_whitespace_only_rows() {
        let styled_output = vec![styled_line("header"), styled_line("   "), styled_line("")];
        let source = TerminalRenderSource {
            session_id: 1,
            state: TerminalState::Running,
            output: "",
            styled_output: &styled_output,
            cursor: None,
        };

        assert_eq!(terminal_render_line_count_for_source(&source, None), 1);
    }

    #[test]
    fn terminal_render_line_count_keeps_blank_cursor_row_visible() {
        let styled_output = vec![styled_line("header"), styled_line("   ")];
        let source = TerminalRenderSource {
            session_id: 1,
            state: TerminalState::Running,
            output: "",
            styled_output: &styled_output,
            cursor: Some(TerminalCursor { line: 1, column: 0 }),
        };

        assert_eq!(terminal_render_line_count_for_source(&source, None), 2);
    }

    #[test]
    fn terminal_render_slice_layout_uses_visible_slice_height_instead_of_full_content_height() {
        let layout = terminal_render_slice_layout(1793, 1734, 59, 18.);

        assert_eq!(layout, TerminalRenderSliceLayout {
            content_height_px: 32274.,
            slice_offset_px: 31212.,
            slice_height_px: 1062.,
        });
    }

    #[test]
    fn terminal_render_slice_layout_falls_back_to_one_line_for_empty_or_invalid_values() {
        let layout = terminal_render_slice_layout(0, 7, 0, 0.);

        assert_eq!(layout, TerminalRenderSliceLayout {
            content_height_px: 1.,
            slice_offset_px: 7.,
            slice_height_px: 1.,
        });
    }

    #[test]
    fn terminal_viewport_slice_range_skips_overscan_and_targets_viewport_rows() {
        let range = terminal_viewport_slice_range(1752, 59, 31756., 842., 18.);

        assert_eq!(range, 12..59);
    }

    #[test]
    fn terminal_viewport_slice_range_clamps_to_one_line_when_values_are_invalid() {
        let range = terminal_viewport_slice_range(12, 0, f32::NAN, 0., 0.);

        assert_eq!(range, 0..1);
    }

    #[test]
    fn terminal_last_non_whitespace_line_index_skips_blank_tail_rows() {
        let lines = vec![
            styled_line("header"),
            styled_line("   "),
            styled_line("menu"),
        ];

        assert_eq!(terminal_last_non_whitespace_line_index(&lines), Some(2));
    }

    #[test]
    fn terminal_excerpt_escapes_control_bytes_and_truncates() {
        assert_eq!(terminal_excerpt("a\tb\nc", 4), "a\\tb\\n...");
    }

    #[test]
    fn cursor_pads_to_column_when_it_is_after_line_content() {
        let theme = ThemeKind::One.palette();
        let session = session_with_styled_line(
            "abc",
            0x112233,
            0x445566,
            Some(TerminalCursor { line: 0, column: 5 }),
        );

        let lines = styled_lines_for_session(&session, theme, true, None, None);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].runs.len(), 2);
        assert_eq!(lines[0].runs[0].text, "abc");
        assert_eq!(lines[0].runs[1].text, " ");
        assert_eq!(lines[0].runs[1].fg, theme.text_primary);
        assert_eq!(lines[0].runs[1].bg, theme.terminal_cursor);
        assert!(lines[0].cells.iter().any(|cell| {
            cell.column == 5 && cell.text == " " && cell.bg == theme.terminal_cursor
        }));
    }

    #[test]
    fn positioned_runs_split_cells_with_zero_width_sequences() {
        let cells = vec![
            TerminalStyledCell {
                column: 0,
                text: "A".to_owned(),
                fg: 0x112233,
                bg: 0x445566,
            },
            TerminalStyledCell {
                column: 1,
                text: "\u{2600}\u{fe0f}".to_owned(),
                fg: 0x112233,
                bg: 0x445566,
            },
            TerminalStyledCell {
                column: 2,
                text: "B".to_owned(),
                fg: 0x112233,
                bg: 0x445566,
            },
        ];

        let runs = positioned_runs_from_cells(&cells);
        assert_eq!(runs.len(), 3);
        assert_eq!(runs[0].text, "A");
        assert_eq!(runs[0].start_column, 0);
        assert_eq!(runs[0].cell_count, 1);
        assert!(runs[0].force_cell_width);
        assert_eq!(runs[1].text, "\u{2600}\u{fe0f}");
        assert_eq!(runs[1].start_column, 1);
        assert_eq!(runs[1].cell_count, 1);
        assert!(!runs[1].force_cell_width);
        assert_eq!(runs[2].text, "B");
        assert_eq!(runs[2].start_column, 2);
        assert_eq!(runs[2].cell_count, 1);
        assert!(runs[2].force_cell_width);
    }

    #[test]
    fn positioned_runs_do_not_force_cell_width_for_powerline_symbols() {
        let cells = vec![
            TerminalStyledCell {
                column: 0,
                text: "\u{e0b0}".to_owned(),
                fg: 0xaabbcc,
                bg: 0x112233,
            },
            TerminalStyledCell {
                column: 1,
                text: "X".to_owned(),
                fg: 0xaabbcc,
                bg: 0x112233,
            },
        ];

        let runs = positioned_runs_from_cells(&cells);
        assert_eq!(runs.len(), 2);
        assert_eq!(runs[0].text, "\u{e0b0}");
        assert!(!runs[0].force_cell_width);
        assert_eq!(runs[1].text, "X");
        assert!(runs[1].force_cell_width);
    }

    #[test]
    fn positioned_runs_keep_cell_width_for_box_drawing_symbols() {
        let cells = vec![
            TerminalStyledCell {
                column: 0,
                text: "\u{2502}".to_owned(),
                fg: 0xaabbcc,
                bg: 0x112233,
            },
            TerminalStyledCell {
                column: 1,
                text: "X".to_owned(),
                fg: 0xaabbcc,
                bg: 0x112233,
            },
        ];

        let runs = positioned_runs_from_cells(&cells);
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].text, "\u{2502}X");
        assert!(runs[0].force_cell_width);
    }

    #[test]
    fn powerline_glyph_is_forced_to_cell_width() {
        let run = PositionedTerminalRun {
            text: "\u{e0b6}".to_owned(),
            fg: 0,
            bg: 0,
            start_column: 7,
            cell_count: 1,
            force_cell_width: false,
        };

        assert!(should_force_powerline(&run));
    }

    #[test]
    fn token_bounds_capture_full_url() {
        let lines = vec!["visit https://example.com/path?q=1 please".to_owned()];
        let point = TerminalGridPosition {
            line: 0,
            column: 12,
        };

        let bounds = terminal_token_bounds(&lines, point);
        assert!(bounds.is_some());
        let (start, end) = bounds.expect("token bounds");
        let selection = TerminalSelection {
            session_id: 1,
            anchor: start,
            head: end,
        };
        let selected = terminal_selection_text(&lines, &selection);
        assert_eq!(selected, "https://example.com/path?q=1");
    }

    #[test]
    fn selection_text_spans_multiple_lines() {
        let lines = vec!["abc".to_owned(), "def".to_owned(), "ghi".to_owned()];
        let selection = TerminalSelection {
            session_id: 1,
            anchor: TerminalGridPosition { line: 0, column: 1 },
            head: TerminalGridPosition { line: 2, column: 2 },
        };

        let selected = terminal_selection_text(&lines, &selection);
        assert_eq!(selected, "bc\ndef\ngh");
    }

    #[test]
    fn line_bounds_capture_entire_line_on_triple_click() {
        let lines = vec!["hello world".to_owned()];
        let point = TerminalGridPosition { line: 0, column: 3 };

        let bounds = terminal_line_bounds(&lines, point);
        assert!(bounds.is_some());
        let (start, end) = bounds.expect("line bounds");
        assert_eq!(start.line, 0);
        assert_eq!(start.column, 0);
        assert_eq!(end.line, 0);
        assert_eq!(end.column, 11);
    }

    #[test]
    fn styled_lines_remap_embedded_default_palette_to_active_theme() {
        let theme = ThemeKind::Gruvbox.palette();
        let session = session_with_styled_line(
            "abc",
            EMBEDDED_TERMINAL_DEFAULT_FG,
            EMBEDDED_TERMINAL_DEFAULT_BG,
            None,
        );

        let lines = styled_lines_for_session(&session, theme, false, None, None);
        assert_eq!(lines.len(), 1);
        assert!(
            lines[0]
                .cells
                .iter()
                .all(|cell| cell.bg == theme.terminal_bg)
        );
        assert!(
            lines[0]
                .cells
                .iter()
                .all(|cell| cell.fg == theme.text_primary)
        );
    }

    #[test]
    fn styled_lines_for_session_range_offsets_cursor_into_visible_slice() {
        let theme = ThemeKind::One.palette();
        let mut session = session_with_styled_line(
            "alpha",
            0x112233,
            0x445566,
            Some(TerminalCursor { line: 2, column: 1 }),
        );
        session.output = "alpha\nbeta\ngamma".to_owned();
        session.styled_output = ["alpha", "beta", "gamma"]
            .into_iter()
            .map(|text| TerminalStyledLine {
                cells: text
                    .chars()
                    .enumerate()
                    .map(|(column, character)| TerminalStyledCell {
                        column,
                        text: character.to_string(),
                        fg: 0x112233,
                        bg: 0x445566,
                    })
                    .collect(),
                runs: vec![TerminalStyledRun {
                    text: text.to_owned(),
                    fg: 0x112233,
                    bg: 0x445566,
                }],
            })
            .collect();

        let lines = styled_lines_for_session_range(&session, theme, true, None, None, 2..3);

        assert_eq!(lines.len(), 1);
        assert!(
            lines[0]
                .cells
                .iter()
                .any(|cell| cell.column == 1 && cell.bg == theme.terminal_cursor)
        );
    }

    #[test]
    fn styled_lines_for_session_range_clips_selection_to_visible_slice() {
        let theme = ThemeKind::One.palette();
        let mut session = session_with_styled_line("alpha", 0x112233, 0x445566, None);
        session.output = "alpha\nbeta\ngamma".to_owned();
        session.styled_output = ["alpha", "beta", "gamma"]
            .into_iter()
            .map(|text| TerminalStyledLine {
                cells: text
                    .chars()
                    .enumerate()
                    .map(|(column, character)| TerminalStyledCell {
                        column,
                        text: character.to_string(),
                        fg: 0x112233,
                        bg: 0x445566,
                    })
                    .collect(),
                runs: vec![TerminalStyledRun {
                    text: text.to_owned(),
                    fg: 0x112233,
                    bg: 0x445566,
                }],
            })
            .collect();
        let selection = TerminalSelection {
            session_id: session.id,
            anchor: TerminalGridPosition { line: 0, column: 2 },
            head: TerminalGridPosition { line: 2, column: 2 },
        };

        let lines =
            styled_lines_for_session_range(&session, theme, false, Some(&selection), None, 1..3);

        assert_eq!(lines.len(), 2);
        assert!(
            lines[0]
                .cells
                .iter()
                .all(|cell| cell.bg == theme.terminal_selection_bg)
        );
        assert!(
            lines[1]
                .cells
                .iter()
                .filter(|cell| cell.column < 2)
                .all(|cell| cell.bg == theme.terminal_selection_bg)
        );
        assert!(
            lines[1]
                .cells
                .iter()
                .filter(|cell| cell.column >= 2)
                .all(|cell| cell.bg == 0x445566)
        );
    }

    #[test]
    fn auto_follow_requires_new_output_and_bottom_position() {
        assert!(should_auto_follow_terminal_output(true, true));
        assert!(!should_auto_follow_terminal_output(true, false));
        assert!(!should_auto_follow_terminal_output(false, true));
    }

    #[test]
    fn sticky_follow_keeps_output_pinned_between_bursts() {
        assert!(terminal_should_follow_output(false, false, false, true));
        assert!(terminal_should_follow_output(false, false, true, false));
        assert!(terminal_should_follow_output(false, true, false, false));
        assert!(terminal_should_follow_output(true, false, false, false));
        assert!(!terminal_should_follow_output(false, false, false, false));
    }

    #[test]
    fn auto_follow_is_disabled_without_new_output() {
        assert!(!should_auto_follow_terminal_output(false, false));
    }

    #[test]
    fn initial_scheduled_follow_pass_allows_one_settle_retry() {
        assert_eq!(
            terminal_scheduled_follow_pass_decision(0, 3, true, false),
            TerminalScheduledFollowPassDecision {
                perform_scroll: false,
                schedule_retry: true,
            }
        );
        assert_eq!(
            terminal_scheduled_follow_pass_decision(0, 3, true, true),
            TerminalScheduledFollowPassDecision {
                perform_scroll: true,
                schedule_retry: true,
            }
        );
    }

    #[test]
    fn later_scheduled_follow_passes_retry_only_after_real_scroll() {
        assert_eq!(
            terminal_scheduled_follow_pass_decision(1, 3, true, false),
            TerminalScheduledFollowPassDecision {
                perform_scroll: false,
                schedule_retry: false,
            }
        );
        assert_eq!(
            terminal_scheduled_follow_pass_decision(1, 3, true, true),
            TerminalScheduledFollowPassDecision {
                perform_scroll: true,
                schedule_retry: true,
            }
        );
        assert_eq!(
            terminal_scheduled_follow_pass_decision(2, 3, true, true),
            TerminalScheduledFollowPassDecision {
                perform_scroll: true,
                schedule_retry: false,
            }
        );
    }

    #[test]
    fn scheduled_follow_passes_stop_without_follow_state() {
        assert_eq!(
            terminal_scheduled_follow_pass_decision(0, 3, false, true),
            TerminalScheduledFollowPassDecision {
                perform_scroll: false,
                schedule_retry: false,
            }
        );
        assert_eq!(
            terminal_scheduled_follow_pass_decision(0, 0, true, true),
            TerminalScheduledFollowPassDecision {
                perform_scroll: false,
                schedule_retry: false,
            }
        );
    }

    #[test]
    fn active_follow_lock_keeps_output_pinned_between_paints() {
        let now = Instant::now();
        let follow_lock_until = Some(now + TERMINAL_OUTPUT_FOLLOW_LOCK_DURATION);
        assert!(terminal_follow_lock_is_active(follow_lock_until, now));
        assert!(should_auto_follow_terminal_output(
            true,
            terminal_follow_lock_is_active(follow_lock_until, now),
        ));
    }

    #[test]
    fn expired_follow_lock_stops_auto_follow_when_not_at_bottom() {
        let now = Instant::now();
        let follow_lock_until = Some(now);
        assert!(!terminal_follow_lock_is_active(follow_lock_until, now));
        assert!(!should_auto_follow_terminal_output(
            true,
            terminal_follow_lock_is_active(follow_lock_until, now),
        ));
    }

    #[test]
    fn interactive_follow_window_stays_active_for_longer_resume_redraws() {
        let now = Instant::now();
        assert!(terminal_interactive_follow_is_active(
            Some(now + INTERACTIVE_TERMINAL_SYNC_WINDOW),
            now,
        ));
        assert!(!terminal_interactive_follow_is_active(Some(now), now));
        assert!(!terminal_interactive_follow_is_active(None, now));
    }

    #[test]
    fn upward_scroll_during_follow_lock_cancels_follow_mode() {
        assert!(terminal_scroll_moved_away_from_bottom(
            Some(px(-120.)),
            px(-80.),
            false,
        ));
        assert!(!terminal_scroll_moved_away_from_bottom(
            Some(px(-120.)),
            px(-160.),
            false,
        ));
        assert!(!terminal_scroll_moved_away_from_bottom(
            Some(px(-120.)),
            px(-80.),
            true,
        ));
    }

    #[test]
    fn scroll_extent_change_detects_growth_and_ignores_steady_repaints() {
        assert!(terminal_scroll_extent_changed(None, px(32.)));
        assert!(terminal_scroll_extent_changed(Some(px(32.)), px(64.)));
        assert!(!terminal_scroll_extent_changed(Some(px(64.)), px(64.)));
        assert!(!terminal_scroll_extent_changed(Some(px(64.)), px(64.5)));
    }

    #[test]
    fn visible_render_signature_ignores_offscreen_scrollback_changes() {
        let source = |header: &str| TerminalRenderSource {
            session_id: 1,
            state: TerminalState::Running,
            output: "",
            styled_output: Box::leak(Box::new(
                std::iter::once(TerminalStyledLine {
                    cells: Vec::new(),
                    runs: vec![TerminalStyledRun {
                        text: header.to_owned(),
                        fg: 0xffffff,
                        bg: 0x000000,
                    }],
                })
                .chain((1..220).map(|index| TerminalStyledLine {
                    cells: Vec::new(),
                    runs: vec![TerminalStyledRun {
                        text: format!("visible line {index:03}"),
                        fg: 0xffffff,
                        bg: 0x000000,
                    }],
                }))
                .collect::<Vec<_>>(),
            )),
            cursor: Some(TerminalCursor {
                line: 219,
                column: 0,
            }),
        };

        let visible_range = 160..220;
        let first = source("header one");
        let second = source("header two");

        assert_eq!(
            terminal_visible_render_signature_for_source(&first, visible_range.clone()),
            terminal_visible_render_signature_for_source(&second, visible_range),
        );
    }

    #[test]
    fn visible_render_signature_changes_when_viewport_changes() {
        let styled_output = Box::leak(Box::new(
            (0..220)
                .map(|index| TerminalStyledLine {
                    cells: Vec::new(),
                    runs: vec![TerminalStyledRun {
                        text: format!("line {index:03}"),
                        fg: 0xffffff,
                        bg: 0x000000,
                    }],
                })
                .collect::<Vec<_>>(),
        ));
        let source = TerminalRenderSource {
            session_id: 1,
            state: TerminalState::Running,
            output: "",
            styled_output,
            cursor: Some(TerminalCursor {
                line: 219,
                column: 0,
            }),
        };

        assert_ne!(
            terminal_visible_render_signature_for_source(&source, 160..220),
            terminal_visible_render_signature_for_source(&source, 159..219),
        );
    }

    #[test]
    fn gui_display_lines_preserve_cursor_relative_prompt_redraws() {
        let mut emulator = TerminalEmulator::with_size(12, 72);
        let initial = concat!(
            "  Would you like to make the following edits?\r\n",
            "\r\n",
            "  crates/arbor-gui/src/app_init.rs (+4 -0)\r\n",
            "    223  -        self.terminal_scroll_handle: ScrollHandle::new(),\r\n",
            "    224  +        terminal_follow_output_until: None,\r\n",
            "    225  +        last_terminal_scroll_offset_y: None,\r\n",
            "\r\n",
            "  1. Yes, proceed (y)\r\n",
            "› 2. Yes, and don't ask again for these files (a)\r\n",
            "  3. No, and tell Codex what to do differently (esc)",
        );
        let redraw = concat!(
            "\x1b[3A",
            "\r\x1b[2K  1. Yes, proceed (y)",
            "\x1b[1B",
            "\r\x1b[2K› 2. Yes, and don't ask again for these files (a)",
            "\x1b[1B",
            "\r\x1b[2K  3. No, and tell Codex what to do differently (esc)",
        );
        emulator.process(initial.as_bytes());
        emulator.process(redraw.as_bytes());

        let snapshot = emulator.snapshot();
        let source = terminal_render_source_for_snapshot(1, TerminalState::Running, &snapshot);
        let rendered = terminal_display_lines_for_source(&source).join("\n");

        assert!(
            rendered.contains("Would you like to make the following edits?"),
            "expected prompt header to survive GUI line conversion: {rendered:?}"
        );
        assert!(
            rendered.contains("terminal_follow_output_until: None"),
            "expected diff content to remain readable in GUI line conversion: {rendered:?}"
        );
        assert!(
            rendered.contains("› 2. Yes, and don't ask again for these files (a)"),
            "expected selected option to survive GUI line conversion: {rendered:?}"
        );
        assert!(
            !rendered.contains("1. Yes, and don't ask again"),
            "unexpected line bleed in GUI line conversion: {rendered:?}"
        );
    }

    #[test]
    fn terminal_display_tail_lines_skip_blank_screen_padding() {
        let source = TerminalRenderSource {
            session_id: 1,
            state: TerminalState::Running,
            output: "",
            styled_output: &[
                TerminalStyledLine {
                    cells: Vec::new(),
                    runs: vec![TerminalStyledRun {
                        text: "header".to_owned(),
                        fg: 0xffffff,
                        bg: 0x000000,
                    }],
                },
                TerminalStyledLine {
                    cells: Vec::new(),
                    runs: vec![TerminalStyledRun {
                        text: "menu".to_owned(),
                        fg: 0xffffff,
                        bg: 0x000000,
                    }],
                },
                TerminalStyledLine {
                    cells: Vec::new(),
                    runs: Vec::new(),
                },
                TerminalStyledLine {
                    cells: Vec::new(),
                    runs: Vec::new(),
                },
            ],
            cursor: Some(TerminalCursor { line: 1, column: 0 }),
        };

        assert_eq!(terminal_display_tail_lines_for_source(&source, 2), vec![
            "header".to_owned(),
            "menu".to_owned()
        ]);
    }

    #[test]
    fn gui_display_lines_preserve_wide_scroll_rows_without_missing_chars() {
        let mut emulator = TerminalEmulator::with_size(48, 120);
        emulator.process(
            b"\x1b[H\x1b[2JFilesystem             Size   Used  Avail Capacity Mounted on\r\n",
        );

        for row in 0..220 {
            let used_gib = (row * 7) % 900 + 50;
            let avail_gib = 1024 - used_gib;
            let capacity = (used_gib * 100) / 1024;
            emulator.process(
                format!(
                    "/dev/disk{row:<3}         1.0Ti  {used_gib:>4}Gi  {avail_gib:>4}Gi    {capacity:>2}%   /Volumes/worktree-{row:03}\r\n"
                )
                .as_bytes(),
            );
        }

        let snapshot = emulator.snapshot();
        let source = terminal_render_source_for_snapshot(1, TerminalState::Running, &snapshot);
        let lines = terminal_display_lines_for_source(&source);
        let expected_last_row =
            "/dev/disk219         1.0Ti   683Gi   341Gi    66%   /Volumes/worktree-219";

        assert!(
            lines.iter().any(|line| {
                line == "Filesystem             Size   Used  Avail Capacity Mounted on"
            }),
            "expected df-like header to survive GUI line conversion: {lines:?}"
        );
        assert_eq!(
            lines.iter().rev().find(|line| !line.is_empty()),
            Some(&expected_last_row.to_owned()),
            "expected final df-like row to survive GUI line conversion without missing chars: {lines:?}"
        );
    }

    #[test]
    fn computes_terminal_grid_size_from_viewport() {
        let result = terminal_grid_size_for_viewport(
            900.,
            380.,
            TERMINAL_CELL_WIDTH_PX,
            TERMINAL_CELL_HEIGHT_PX,
        );
        assert_eq!(result, Some((20, 100)));
    }
}
