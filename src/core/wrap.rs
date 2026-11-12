// wrap module - soft wrapping of buffer lines into visual rows, shared by tui and gui.
// Wrapping is display only: the buffer keeps its real lines, nothing is inserted into the file.
use crate::core::selection::TextPosition;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

// A visual row: a buffer line and the wrapped row inside it.
// Ordered by line first, then row, so it can be compared directly.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct RowPos {
    pub line: usize,
    pub row: usize,
}

// Grapheme index where each visual row of `line` starts (the first entry is always 0).
// Breaks after the last whitespace that fits (word wrap), or mid word when a word is
// wider than the row. Whitespace at a break is allowed to hang past the edge.
pub fn row_starts(line: &str, cols: usize) -> Vec<usize> {
    let mut starts = vec![0];
    if cols == 0 {
        return starts;
    }

    let mut widths: Vec<usize> = Vec::new();
    let mut row_start = 0;
    let mut row_width = 0;
    let mut after_space: Option<usize> = None;

    for (i, g) in line.graphemes(true).enumerate() {
        let w = UnicodeWidthStr::width(g);
        let is_space = g.chars().all(char::is_whitespace);
        widths.push(w);

        if row_width + w > cols && i > row_start && !is_space {
            let brk = match after_space {
                Some(b) if b > row_start => b,
                _ => i,
            };
            starts.push(brk);
            row_start = brk;
            row_width = widths[brk..i].iter().sum();
            // A wide character may still not fit after moving the word down
            if row_width + w > cols && i > row_start {
                starts.push(i);
                row_start = i;
                row_width = 0;
            }
            after_space = None;
        }

        row_width += w;
        if is_space {
            after_space = Some(i + 1);
        }
    }

    starts
}

// Index of the row containing grapheme column `col`.
// A column exactly on a break belongs to the row that starts there.
pub fn row_of(starts: &[usize], col: usize) -> usize {
    starts.partition_point(|&s| s <= col).saturating_sub(1)
}

// Grapheme range [start, end) of row `row`, given the line's grapheme count.
pub fn row_range(starts: &[usize], row: usize, line_len: usize) -> (usize, usize) {
    let start = starts.get(row).copied().unwrap_or(line_len);
    let end = starts.get(row + 1).copied().unwrap_or(line_len);
    (start, end)
}

// Last column the caret can take on `row` without being shown on the next row.
pub fn row_last_col(starts: &[usize], row: usize, line_len: usize) -> usize {
    match starts.get(row + 1) {
        Some(&next) => next.saturating_sub(1).max(starts[row]),
        None => line_len,
    }
}

// Row of grapheme column `col` and its display column inside that row.
pub fn locate(line: &str, cols: usize, col: usize) -> (usize, usize) {
    let graphemes: Vec<&str> = line.graphemes(true).collect();
    let starts = row_starts(line, cols);
    let col = col.min(graphemes.len());
    let row = row_of(&starts, col);
    let x = graphemes[starts[row]..col]
        .iter()
        .map(|g| UnicodeWidthStr::width(*g))
        .sum();
    (row, x)
}

// Grapheme column on `row` closest to display column `x` (snaps to the nearer side of a character).
pub fn col_at_x(line: &str, cols: usize, row: usize, x: usize) -> usize {
    let graphemes: Vec<&str> = line.graphemes(true).collect();
    let starts = row_starts(line, cols);
    let row = row.min(starts.len() - 1);
    let start = starts[row];
    let last = row_last_col(&starts, row, graphemes.len());

    let mut acc = 0;
    for (i, g) in graphemes.iter().enumerate().take(last).skip(start) {
        let w = UnicodeWidthStr::width(*g);
        if x < acc + w.div_ceil(2) {
            return i;
        }
        acc += w;
    }
    last
}

fn line_at(lines: &[String], idx: usize) -> &str {
    lines.get(idx).map(String::as_str).unwrap_or("")
}

pub fn row_count(lines: &[String], line: usize, cols: usize) -> usize {
    row_starts(line_at(lines, line), cols).len()
}

// Visual row of a text position.
pub fn row_pos_of(lines: &[String], cols: usize, pos: TextPosition) -> RowPos {
    let starts = row_starts(line_at(lines, pos.line), cols);
    RowPos {
        line: pos.line,
        row: row_of(&starts, pos.column),
    }
}

// Keep a row position inside its line (rows disappear when a line gets shorter).
pub fn clamp_row(lines: &[String], cols: usize, pos: RowPos) -> RowPos {
    let line = pos.line.min(lines.len().saturating_sub(1));
    let rows = row_count(lines, line, cols);
    RowPos {
        line,
        row: pos.row.min(rows - 1),
    }
}

// Number of visual rows from `top` down to `target`, None if target is above top.
// Stops counting once `limit` is passed.
pub fn rows_between(
    lines: &[String],
    cols: usize,
    top: RowPos,
    target: RowPos,
    limit: usize,
) -> Option<usize> {
    if target < top {
        return None;
    }
    if target.line == top.line {
        return Some(target.row - top.row);
    }
    let mut n = row_count(lines, top.line, cols).saturating_sub(top.row);
    for line in top.line + 1..target.line {
        if n > limit {
            return Some(n);
        }
        n += row_count(lines, line, cols);
    }
    Some(n + target.row)
}

// Move `n` visual rows up, stopping at the first row of the document.
pub fn step_back(lines: &[String], cols: usize, mut pos: RowPos, mut n: usize) -> RowPos {
    while n > 0 {
        if pos.row >= n {
            pos.row -= n;
            return pos;
        }
        if pos.line == 0 {
            pos.row = 0;
            return pos;
        }
        n -= pos.row + 1;
        pos.line -= 1;
        pos.row = row_count(lines, pos.line, cols) - 1;
    }
    pos
}

// Move `n` visual rows down, stopping at the last row of the document.
pub fn step_forward(lines: &[String], cols: usize, mut pos: RowPos, mut n: usize) -> RowPos {
    let last_line = lines.len().saturating_sub(1);
    while n > 0 {
        let rows = row_count(lines, pos.line, cols);
        let left_in_line = rows - 1 - pos.row.min(rows - 1);
        if left_in_line >= n {
            pos.row += n;
            return pos;
        }
        if pos.line >= last_line {
            pos.row = rows - 1;
            return pos;
        }
        n -= left_in_line + 1;
        pos.line += 1;
        pos.row = 0;
    }
    pos
}

// Lowest the viewport top may go: the last row of the document sits on the bottom row.
pub fn max_top(lines: &[String], cols: usize, visible_rows: usize) -> RowPos {
    let last_line = lines.len().saturating_sub(1);
    let last = RowPos {
        line: last_line,
        row: row_count(lines, last_line, cols) - 1,
    };
    step_back(lines, cols, last, visible_rows.saturating_sub(1))
}

// Scroll the viewport by `delta` visual rows (negative = up).
pub fn scroll_by(
    lines: &[String],
    cols: usize,
    top: RowPos,
    delta: isize,
    visible_rows: usize,
) -> RowPos {
    let top = clamp_row(lines, cols, top);
    if delta < 0 {
        step_back(lines, cols, top, delta.unsigned_abs())
    } else {
        let max = max_top(lines, cols, visible_rows);
        step_forward(lines, cols, top, delta as usize).min(max)
    }
}

// Viewport top that keeps `target` visible, moving as little as possible.
pub fn ensure_visible(
    lines: &[String],
    cols: usize,
    top: RowPos,
    visible_rows: usize,
    target: RowPos,
) -> RowPos {
    let top = clamp_row(lines, cols, top);
    let visible_rows = visible_rows.max(1);
    match rows_between(lines, cols, top, target, visible_rows) {
        None => target,
        Some(n) if n < visible_rows => top,
        Some(_) => step_back(lines, cols, target, visible_rows - 1),
    }
}
