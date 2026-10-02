// search module for finding text matches, shared by tui and gui

// Stores all search match locations
#[derive(Clone, Debug)]
pub struct SearchMatch {
    pub line: usize,
    pub column: usize,
    pub length: usize,
}

pub struct SearchState {
    pub _query: String,
    pub matches: Vec<SearchMatch>,
    pub current_match_idx: usize,
}

impl SearchState {
    pub fn new(_query: String, matches: Vec<SearchMatch>) -> Self {
        Self {
            _query,
            matches,
            current_match_idx: 0,
        }
    }

    pub fn next_match(&mut self) {
        if !self.matches.is_empty() {
            self.current_match_idx = (self.current_match_idx + 1) % self.matches.len();
        }
    }

    pub fn prev_match(&mut self) {
        if !self.matches.is_empty() {
            self.current_match_idx = if self.current_match_idx == 0 {
                self.matches.len() - 1
            } else {
                self.current_match_idx - 1
            };
        }
    }

    pub fn current_match(&self) -> Option<&SearchMatch> {
        self.matches.get(self.current_match_idx)
    }
}

// Case-insensitive search, columns and length are in characters
pub fn find_all_occurrences(lines: &[String], query: &str) -> Vec<SearchMatch> {
    let query_lower: Vec<char> = query.chars().map(fold_case).collect();
    let mut matches = Vec::new();

    if query_lower.is_empty() {
        return matches;
    }

    for (line_idx, line) in lines.iter().enumerate() {
        let line_lower: Vec<char> = line.chars().map(fold_case).collect();
        if line_lower.len() < query_lower.len() {
            continue;
        }

        for column in 0..=line_lower.len() - query_lower.len() {
            if line_lower[column..column + query_lower.len()] == query_lower[..] {
                matches.push(SearchMatch {
                    line: line_idx,
                    column,
                    length: query_lower.len(),
                });
            }
        }
    }

    matches
}

pub fn find_closest_match(matches: &[SearchMatch], line: usize, col: usize) -> usize {
    let mut closest_idx = 0;
    let mut min_distance = usize::MAX;

    for (idx, m) in matches.iter().enumerate() {
        // Calculate distance (prioritize line, then column)
        let distance = if m.line == line {
            if m.column >= col {
                m.column - col
            } else {
                usize::MAX / 2 + (col - m.column)
            }
        } else if m.line > line {
            (m.line - line) * 1000 + m.column
        } else {
            usize::MAX - (line - m.line) * 1000
        };

        if distance < min_distance {
            min_distance = distance;
            closest_idx = idx;
        }
    }

    closest_idx
}

// lowercase one char without changing the char count, so columns stay aligned
fn fold_case(c: char) -> char {
    c.to_lowercase().next().unwrap_or(c)
}
