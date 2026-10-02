// buffer module responsible for buffer size
#[derive(Clone)]
pub struct Buffer {
    pub lines: Vec<String>,
}

impl Buffer {
    // handle loading a file
    pub fn from_string(content: String) -> Self {
        let mut lines: Vec<String> = content.lines().map(|line| line.to_string()).collect();

        // Ensure there is at least one line if the file is empty
        if lines.is_empty() {
            lines.push(String::new());
        }
        
        // Add buffer space for expansion (500 empty lines after content)
        for _ in 0..500 {
            lines.push(String::new());
        }
        
        Self { lines }
    }

    // content written to disk, the trailing empty buffer lines are left out
    pub fn to_file_content(&self) -> String {
        let end = self
            .lines
            .iter()
            .rposition(|line| !line.is_empty())
            .map_or(0, |last_line| last_line + 1);
        self.lines[..end].join("\n")
    }
}

impl Default for Buffer {
    fn default() -> Self {
        let mut lines = Vec::new();
        // generate 500 lines of Buffer
        for _ in 0..500 {
            lines.push(String::new());
        }
        Self { lines }
    }
}