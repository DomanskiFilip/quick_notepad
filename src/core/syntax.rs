// Syntax highlighting module with vibrant colors - core implementation
// Colors are represented as RGB tuples for cross-compatibility between TUI and GUI

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TokenType {
    Keyword,
    Type,
    String,
    Number,
    Comment,
    Operator,
    Punctuation,
    Variable,
    Constant,
    Macro,
    Attribute,
    Normal,
}

impl TokenType {
    // Return RGB color values for cross-platform compatibility
    pub fn rgb(&self) -> (u8, u8, u8) {
        match self {
            TokenType::Keyword => (255, 140, 0),      // Deep Orange
            TokenType::Type => (255, 215, 0),         // Gold (matches theme)
            TokenType::String => (144, 238, 144),     // Light Green
            TokenType::Number => (255, 105, 180),     // Hot Pink
            TokenType::Comment => (128, 128, 128),    // Grey
            TokenType::Operator => (255, 69, 0),      // Red-Orange
            TokenType::Punctuation => (200, 200, 200), // Light Grey
            TokenType::Variable => (255, 255, 255),   // White
            TokenType::Constant => (255, 165, 0),     // Orange
            TokenType::Macro => (186, 85, 211),       // Medium Orchid
            TokenType::Attribute => (147, 112, 219),  // Medium Purple
            TokenType::Normal => (255, 255, 255),     // White
        }
    }
}

#[derive(Debug, Clone)]
pub struct Token {
    pub text: String,
    pub token_type: TokenType,
}

pub struct SyntaxHighlighter {
    file_type: Option<String>,
}

impl SyntaxHighlighter {
    pub fn new(file_type: Option<String>) -> Self {
        Self { file_type }
    }

    pub fn highlight_line(&self, line: &str) -> Vec<Token> {
        match self.file_type.as_deref() {
            // Rust
            Some("Rust") => self.highlight_rust(line),
            
            // Python
            Some("Python") => self.highlight_python(line),
            
            // JavaScript/TypeScript
            Some("JavaScript") | Some("TypeScript") | Some("React JSX") | Some("React TSX") => {
                self.highlight_javascript(line)
            }
            
            // C-family
            Some("C") | Some("C++") | Some("C#") => self.highlight_c(line),
            
            // Java/JVM languages
            Some("Java") | Some("Kotlin") | Some("Scala") => self.highlight_java(line),
            
            // Go
            Some("Go") => self.highlight_go(line),
            
            // Ruby
            Some("Ruby") => self.highlight_ruby(line),
            
            // PHP
            Some("PHP") => self.highlight_php(line),
            
            // Swift
            Some("Swift") => self.highlight_swift(line),
            
            // Shell scripts
            Some("Shell Script") | Some("Bash Script") | Some("Zsh Script") => {
                self.highlight_shell(line)
            }
            
            // Web markup
            Some("HTML") | Some("XML") | Some("Vue") => self.highlight_html(line),
            Some("CSS") | Some("Sass") => self.highlight_css(line),
            
            // Data formats
            Some("JSON") => self.highlight_json(line),
            Some("TOML") => self.highlight_toml(line),
            Some("YAML") => self.highlight_yaml(line),
            
            // Configuration files
            Some("Config") | Some("Config File") | Some("Environment") => {
                self.highlight_config(line)
            }
            
            // SQL
            Some("SQL Query") => self.highlight_sql(line),
            
            // Markdown
            Some("Markdown") => self.highlight_markdown(line),
            
            // Other languages that can use generic C-like highlighting
            Some("Dart") | Some("Zig") | Some("Nim") => self.highlight_c(line),
            
            // Default
            _ => vec![Token {
                text: line.to_string(),
                token_type: TokenType::Normal,
            }],
        }
    }

    fn highlight_rust(&self, line: &str) -> Vec<Token> {
        let keywords = [
            "fn", "let", "mut", "const", "static", "if", "else", "match", "for", "while",
            "loop", "break", "continue", "return", "pub", "mod", "use", "impl", "trait",
            "struct", "enum", "type", "where", "async", "await", "move", "ref", "self",
            "Self", "super", "crate", "as", "unsafe", "extern", "in",
        ];

        let types = [
            "i8", "i16", "i32", "i64", "i128", "isize", "u8", "u16", "u32", "u64", "u128",
            "usize", "f32", "f64", "bool", "char", "str", "String", "Vec", "Option",
            "Result", "Box", "Rc", "Arc", "Cell", "RefCell",
        ];

        self.tokenize_line(line, &keywords, &types, "//", &["/*"], &["*/"])
    }

    fn highlight_python(&self, line: &str) -> Vec<Token> {
        let keywords = [
            "def", "class", "if", "elif", "else", "for", "while", "break", "continue",
            "return", "import", "from", "as", "try", "except", "finally", "raise",
            "with", "lambda", "yield", "async", "await", "pass", "None", "True", "False",
            "and", "or", "not", "in", "is", "global", "nonlocal",
        ];

        let types = ["int", "str", "float", "bool", "list", "dict", "tuple", "set"];

        self.tokenize_line(line, &keywords, &types, "#", &[], &[])
    }

    fn highlight_javascript(&self, line: &str) -> Vec<Token> {
        let keywords = [
            "function", "const", "let", "var", "if", "else", "for", "while", "do",
            "switch", "case", "break", "continue", "return", "class", "extends",
            "import", "export", "from", "async", "await", "try", "catch", "finally",
            "throw", "new", "this", "super", "typeof", "instanceof", "delete",
            "void", "null", "undefined", "true", "false",
        ];

        let types = [
            "Array", "Object", "String", "Number", "Boolean", "Function", "Promise",
            "Map", "Set", "WeakMap", "WeakSet",
        ];

        self.tokenize_line(line, &keywords, &types, "//", &["/*"], &["*/"])
    }

    fn highlight_c(&self, line: &str) -> Vec<Token> {
        let keywords = [
            "if", "else", "while", "for", "do", "switch", "case", "break", "continue",
            "return", "goto", "typedef", "struct", "union", "enum", "sizeof", "static",
            "extern", "const", "volatile", "inline", "auto", "register",
        ];

        let types = [
            "void", "int", "char", "short", "long", "float", "double", "signed",
            "unsigned", "size_t", "uint8_t", "uint16_t", "uint32_t", "uint64_t",
            "int8_t", "int16_t", "int32_t", "int64_t",
        ];

        self.tokenize_line(line, &keywords, &types, "//", &["/*"], &["*/"])
    }

    fn highlight_json(&self, line: &str) -> Vec<Token> {
        let mut tokens = Vec::new();
        let mut current = String::new();
        let mut in_string = false;
        let mut chars = line.chars().peekable();

        while let Some(ch) = chars.next() {
            if ch == '"' {
                if !current.is_empty() {
                    tokens.push(Token {
                        text: current.clone(),
                        token_type: if in_string {
                            TokenType::String
                        } else {
                            TokenType::Normal
                        },
                    });
                    current.clear();
                }
                current.push(ch);
                in_string = !in_string;
                if !in_string {
                    tokens.push(Token {
                        text: current.clone(),
                        token_type: TokenType::String,
                    });
                    current.clear();
                }
            } else if in_string {
                current.push(ch);
            } else if ch.is_numeric() || (ch == '-' && current.is_empty()) {
                current.push(ch);
                while let Some(&next) = chars.peek() {
                    if next.is_numeric() || next == '.' || next == 'e' || next == 'E' || next == '-' || next == '+' {
                        current.push(chars.next().unwrap());
                    } else {
                        break;
                    }
                }
                tokens.push(Token {
                    text: current.clone(),
                    token_type: TokenType::Number,
                });
                current.clear();
            } else if ch.is_alphabetic() {
                current.push(ch);
                while let Some(&next) = chars.peek() {
                    if next.is_alphabetic() {
                        current.push(chars.next().unwrap());
                    } else {
                        break;
                    }
                }
                let token_type = match current.as_str() {
                    "true" | "false" | "null" => TokenType::Keyword,
                    _ => TokenType::Normal,
                };
                tokens.push(Token {
                    text: current.clone(),
                    token_type,
                });
                current.clear();
            } else if ch == '{' || ch == '}' || ch == '[' || ch == ']' || ch == ':' || ch == ',' {
                if !current.is_empty() {
                    tokens.push(Token {
                        text: current.clone(),
                        token_type: TokenType::Normal,
                    });
                    current.clear();
                }
                tokens.push(Token {
                    text: ch.to_string(),
                    token_type: TokenType::Punctuation,
                });
            } else {
                current.push(ch);
            }
        }

        if !current.is_empty() {
            tokens.push(Token {
                text: current,
                token_type: if in_string {
                    TokenType::String
                } else {
                    TokenType::Normal
                },
            });
        }

        if tokens.is_empty() {
            tokens.push(Token {
                text: line.to_string(),
                token_type: TokenType::Normal,
            });
        }

        tokens
    }

    fn highlight_toml(&self, line: &str) -> Vec<Token> {
        let trimmed = line.trim_start();
        
        if trimmed.starts_with('#') {
            return vec![Token {
                text: line.to_string(),
                token_type: TokenType::Comment,
            }];
        }

        if trimmed.starts_with('[') {
            return vec![Token {
                text: line.to_string(),
                token_type: TokenType::Keyword,
            }];
        }

        if let Some(eq_pos) = line.find('=') {
            let mut tokens = Vec::new();
            let key = &line[..eq_pos];
            tokens.push(Token {
                text: key.to_string(),
                token_type: TokenType::Variable,
            });
            tokens.push(Token {
                text: "=".to_string(),
                token_type: TokenType::Operator,
            });

            let value = line[eq_pos + 1..].trim_start();
            let value_type = if value.starts_with('"') || value.starts_with('\'') {
                TokenType::String
            } else if value.parse::<f64>().is_ok() {
                TokenType::Number
            } else if value == "true" || value == "false" {
                TokenType::Keyword
            } else {
                TokenType::Normal
            };

            tokens.push(Token {
                text: value.to_string(),
                token_type: value_type,
            });

            return tokens;
        }

        vec![Token {
            text: line.to_string(),
            token_type: TokenType::Normal,
        }]
    }

    fn highlight_markdown(&self, line: &str) -> Vec<Token> {
        let trimmed = line.trim_start();

        if trimmed.starts_with('#') {
            return vec![Token {
                text: line.to_string(),
                token_type: TokenType::Keyword,
            }];
        }

        if trimmed.starts_with("```") {
            return vec![Token {
                text: line.to_string(),
                token_type: TokenType::String,
            }];
        }

        if trimmed.starts_with('-') || trimmed.starts_with('*') || trimmed.starts_with('+') {
            return vec![Token {
                text: line.to_string(),
                token_type: TokenType::Operator,
            }];
        }

        vec![Token {
            text: line.to_string(),
            token_type: TokenType::Normal,
        }]
    }

    fn highlight_java(&self, line: &str) -> Vec<Token> {
        let keywords = [
            "public", "private", "protected", "static", "final", "abstract", "class",
            "interface", "extends", "implements", "import", "package", "new", "return",
            "if", "else", "for", "while", "do", "switch", "case", "break", "continue",
            "try", "catch", "finally", "throw", "throws", "void", "this", "super",
            "synchronized", "volatile", "transient", "native", "strictfp", "enum",
            "assert", "instanceof", "default", "true", "false", "null",
        ];

        let types = [
            "int", "long", "short", "byte", "char", "float", "double", "boolean",
            "String", "Integer", "Long", "Short", "Byte", "Character", "Float",
            "Double", "Boolean", "Object", "List", "ArrayList", "Map", "HashMap",
            "Set", "HashSet", "Collection",
        ];

        self.tokenize_line(line, &keywords, &types, "//", &["/*"], &["*/"])
    }

    fn highlight_go(&self, line: &str) -> Vec<Token> {
        let keywords = [
            "func", "var", "const", "type", "struct", "interface", "package", "import",
            "if", "else", "for", "range", "switch", "case", "break", "continue",
            "return", "defer", "go", "select", "chan", "map", "true", "false",
            "nil", "goto", "fallthrough", "default",
        ];

        let types = [
            "int", "int8", "int16", "int32", "int64", "uint", "uint8", "uint16",
            "uint32", "uint64", "float32", "float64", "string", "bool", "byte",
            "rune", "error", "complex64", "complex128",
        ];

        self.tokenize_line(line, &keywords, &types, "//", &["/*"], &["*/"])
    }

    fn highlight_ruby(&self, line: &str) -> Vec<Token> {
        let keywords = [
            "def", "end", "class", "module", "if", "elsif", "else", "unless", "case",
            "when", "while", "until", "for", "break", "next", "redo", "retry", "return",
            "yield", "super", "self", "nil", "true", "false", "and", "or", "not",
            "begin", "rescue", "ensure", "raise", "require", "include", "extend",
            "attr_reader", "attr_writer", "attr_accessor", "alias", "undef",
        ];

        let types = ["Array", "Hash", "String", "Integer", "Float", "Symbol", "Proc"];

        self.tokenize_line(line, &keywords, &types, "#", &[], &[])
    }

    fn highlight_php(&self, line: &str) -> Vec<Token> {
        let keywords = [
            "function", "class", "public", "private", "protected", "static", "final",
            "abstract", "interface", "extends", "implements", "new", "return", "if",
            "else", "elseif", "for", "foreach", "while", "do", "switch", "case",
            "break", "continue", "try", "catch", "finally", "throw", "namespace",
            "use", "const", "var", "echo", "print", "true", "false", "null",
            "require", "require_once", "include", "include_once", "as",
        ];

        let types = [
            "int", "string", "bool", "float", "array", "object", "mixed", "void",
            "callable", "iterable",
        ];

        self.tokenize_line(line, &keywords, &types, "//", &["/*"], &["*/"])
    }

    fn highlight_swift(&self, line: &str) -> Vec<Token> {
        let keywords = [
            "func", "var", "let", "class", "struct", "enum", "protocol", "extension",
            "if", "else", "guard", "switch", "case", "for", "while", "repeat", "break",
            "continue", "return", "throw", "throws", "rethrows", "try", "catch",
            "import", "public", "private", "fileprivate", "internal", "static", "final",
            "override", "mutating", "nonmutating", "lazy", "weak", "unowned", "self",
            "Self", "super", "init", "deinit", "subscript", "true", "false", "nil",
            "as", "is", "in", "inout", "associatedtype", "typealias",
        ];

        let types = [
            "Int", "Int8", "Int16", "Int32", "Int64", "UInt", "UInt8", "UInt16",
            "UInt32", "UInt64", "Float", "Double", "String", "Bool", "Character",
            "Array", "Dictionary", "Set", "Optional", "Any", "AnyObject", "Void",
        ];

        self.tokenize_line(line, &keywords, &types, "//", &["/*"], &["*/"])
    }

    fn highlight_shell(&self, line: &str) -> Vec<Token> {
        let trimmed = line.trim_start();

        if trimmed.starts_with('#') {
            return vec![Token {
                text: line.to_string(),
                token_type: TokenType::Comment,
            }];
        }

        let keywords = [
            "if", "then", "else", "elif", "fi", "case", "esac", "for", "while",
            "until", "do", "done", "function", "select", "time", "in", "break",
            "continue", "return", "exit", "export", "local", "readonly", "declare",
            "eval", "exec", "shift", "test", "source", "alias", "unalias",
        ];

        let types = ["true", "false"];

        self.tokenize_line(line, &keywords, &types, "#", &[], &[])
    }

    fn highlight_html(&self, line: &str) -> Vec<Token> {
        let mut tokens = Vec::new();
        let mut current = String::new();
        let mut in_tag = false;
        let mut in_string = false;
        let mut string_char = '"';

        for ch in line.chars() {
            if ch == '"' || ch == '\'' {
                if !in_string {
                    in_string = true;
                    string_char = ch;
                    current.push(ch);
                } else if ch == string_char {
                    current.push(ch);
                    tokens.push(Token {
                        text: current.clone(),
                        token_type: TokenType::String,
                    });
                    current.clear();
                    in_string = false;
                } else {
                    current.push(ch);
                }
                continue;
            }

            if in_string {
                current.push(ch);
                continue;
            }

            if ch == '<' {
                if !current.is_empty() {
                    tokens.push(Token {
                        text: current.clone(),
                        token_type: TokenType::Normal,
                    });
                    current.clear();
                }
                in_tag = true;
                current.push(ch);
            } else if ch == '>' && in_tag {
                current.push(ch);
                tokens.push(Token {
                    text: current.clone(),
                    token_type: TokenType::Keyword,
                });
                current.clear();
                in_tag = false;
            } else {
                current.push(ch);
            }
        }

        if !current.is_empty() {
            tokens.push(Token {
                text: current,
                token_type: if in_tag {
                    TokenType::Keyword
                } else if in_string {
                    TokenType::String
                } else {
                    TokenType::Normal
                },
            });
        }

        if tokens.is_empty() {
            tokens.push(Token {
                text: line.to_string(),
                token_type: TokenType::Normal,
            });
        }

        tokens
    }

    fn highlight_css(&self, line: &str) -> Vec<Token> {
        let trimmed = line.trim_start();

        if trimmed.starts_with("/*") || trimmed.starts_with("//") {
            return vec![Token {
                text: line.to_string(),
                token_type: TokenType::Comment,
            }];
        }

        if line.contains(':') && !line.trim_start().starts_with('@') {
            let mut tokens = Vec::new();
            if let Some(colon_pos) = line.find(':') {
                let property = &line[..colon_pos];
                tokens.push(Token {
                    text: property.to_string(),
                    token_type: TokenType::Variable,
                });
                tokens.push(Token {
                    text: line[colon_pos..].to_string(),
                    token_type: TokenType::Normal,
                });
                return tokens;
            }
        }

        if line.contains('{') || line.contains('}') || trimmed.starts_with('@') {
            return vec![Token {
                text: line.to_string(),
                token_type: TokenType::Keyword,
            }];
        }

        vec![Token {
            text: line.to_string(),
            token_type: TokenType::Normal,
        }]
    }

    fn highlight_yaml(&self, line: &str) -> Vec<Token> {
        let trimmed = line.trim_start();

        if trimmed.starts_with('#') {
            return vec![Token {
                text: line.to_string(),
                token_type: TokenType::Comment,
            }];
        }

        if let Some(colon_pos) = line.find(':') {
            let mut tokens = Vec::new();
            let key = &line[..colon_pos];
            tokens.push(Token {
                text: key.to_string(),
                token_type: TokenType::Variable,
            });
            tokens.push(Token {
                text: ":".to_string(),
                token_type: TokenType::Operator,
            });

            let value = line[colon_pos + 1..].trim_start();
            if !value.is_empty() {
                let value_type = if value.starts_with('"') || value.starts_with('\'') {
                    TokenType::String
                } else if value.parse::<f64>().is_ok() {
                    TokenType::Number
                } else if value == "true" || value == "false" || value == "null" {
                    TokenType::Keyword
                } else {
                    TokenType::Normal
                };

                tokens.push(Token {
                    text: format!(" {}", value),
                    token_type: value_type,
                });
            }

            return tokens;
        }

        if trimmed.starts_with('-') {
            return vec![Token {
                text: line.to_string(),
                token_type: TokenType::Operator,
            }];
        }

        vec![Token {
            text: line.to_string(),
            token_type: TokenType::Normal,
        }]
    }

    fn highlight_sql(&self, line: &str) -> Vec<Token> {
        let keywords = [
            "SELECT", "FROM", "WHERE", "INSERT", "UPDATE", "DELETE", "CREATE",
            "DROP", "ALTER", "TABLE", "DATABASE", "INDEX", "VIEW", "JOIN", "INNER",
            "LEFT", "RIGHT", "OUTER", "ON", "AS", "AND", "OR", "NOT", "IN", "LIKE",
            "BETWEEN", "IS", "NULL", "ORDER", "BY", "GROUP", "HAVING", "LIMIT",
            "OFFSET", "UNION", "DISTINCT", "COUNT", "SUM", "AVG", "MAX", "MIN",
            "PRIMARY", "KEY", "FOREIGN", "REFERENCES", "CONSTRAINT", "DEFAULT",
            "AUTO_INCREMENT", "CASCADE", "SET", "VALUES", "INTO",
        ];

        let types = [
            "INT", "INTEGER", "VARCHAR", "CHAR", "TEXT", "DATE", "DATETIME",
            "TIMESTAMP", "BOOLEAN", "FLOAT", "DOUBLE", "DECIMAL", "BLOB",
        ];

        self.tokenize_line(line, &keywords, &types, "--", &["/*"], &["*/"])
    }

    fn highlight_config(&self, line: &str) -> Vec<Token> {
        let trimmed = line.trim_start();

        if trimmed.starts_with('#') || trimmed.starts_with(';') || trimmed.starts_with("//") {
            return vec![Token {
                text: line.to_string(),
                token_type: TokenType::Comment,
            }];
        }

        let mut tokens = Vec::new();
        let mut current = String::new();
        let mut in_string = false;
        let mut string_char = '"';
        let mut chars = line.chars().peekable();

        while let Some(ch) = chars.next() {
            if !in_string && (ch == '#' || ch == ';') {
                if !current.is_empty() {
                    tokens.push(Token {
                        text: current.clone(),
                        token_type: TokenType::Normal,
                    });
                    current.clear();
                }
                let rest: String = std::iter::once(ch).chain(chars).collect();
                tokens.push(Token {
                    text: rest,
                    token_type: TokenType::Comment,
                });
                break;
            }

            if ch == '"' || ch == '\'' {
                if !in_string {
                    if !current.is_empty() {
                        tokens.push(Token {
                            text: current.clone(),
                            token_type: TokenType::Normal,
                        });
                        current.clear();
                    }
                    in_string = true;
                    string_char = ch;
                    current.push(ch);
                } else if ch == string_char {
                    current.push(ch);
                    tokens.push(Token {
                        text: current.clone(),
                        token_type: TokenType::String,
                    });
                    current.clear();
                    in_string = false;
                } else {
                    current.push(ch);
                }
                continue;
            }

            if in_string {
                current.push(ch);
                continue;
            }

            if ch == '{' || ch == '}' || ch == '[' || ch == ']' || ch == '(' || ch == ')' {
                if !current.is_empty() {
                    tokens.push(Token {
                        text: current.clone(),
                        token_type: TokenType::Normal,
                    });
                    current.clear();
                }
                tokens.push(Token {
                    text: ch.to_string(),
                    token_type: TokenType::Keyword,
                });
                continue;
            }

            if ch == '=' || ch == ':' {
                if !current.is_empty() {
                    tokens.push(Token {
                        text: current.clone(),
                        token_type: TokenType::Variable,
                    });
                    current.clear();
                }
                tokens.push(Token {
                    text: ch.to_string(),
                    token_type: TokenType::Operator,
                });
                continue;
            }

            if ch == '[' {
                if !current.is_empty() {
                    tokens.push(Token {
                        text: current.clone(),
                        token_type: TokenType::Normal,
                    });
                    current.clear();
                }
                current.push(ch);
                while let Some(&next_ch) = chars.peek() {
                    current.push(chars.next().unwrap());
                    if next_ch == ']' {
                        break;
                    }
                }
                tokens.push(Token {
                    text: current.clone(),
                    token_type: TokenType::Type,
                });
                current.clear();
                continue;
            }

            current.push(ch);
        }

        if !current.is_empty() {
            let token_type = if current.trim().parse::<f64>().is_ok() {
                TokenType::Number
            } else if current.trim() == "true" || current.trim() == "false" 
                    || current.trim() == "True" || current.trim() == "False"
                    || current.trim() == "TRUE" || current.trim() == "FALSE"
                    || current.trim() == "yes" || current.trim() == "no"
                    || current.trim() == "on" || current.trim() == "off" {
                TokenType::Keyword
            } else if in_string {
                TokenType::String
            } else {
                TokenType::Normal
            };

            tokens.push(Token {
                text: current,
                token_type,
            });
        }

        if tokens.is_empty() {
            tokens.push(Token {
                text: line.to_string(),
                token_type: TokenType::Normal,
            });
        }

        tokens
    }

    fn tokenize_line(
        &self,
        line: &str,
        keywords: &[&str],
        types: &[&str],
        single_comment: &str,
        multi_comment_start: &[&str],
        _multi_comment_end: &[&str],
    ) -> Vec<Token> {
        if let Some(comment_pos) = line.find(single_comment) {
            let mut tokens = Vec::new();
            if comment_pos > 0 {
                tokens.extend(self.tokenize_code(&line[..comment_pos], keywords, types));
            }
            tokens.push(Token {
                text: line[comment_pos..].to_string(),
                token_type: TokenType::Comment,
            });
            return tokens;
        }

        for start_marker in multi_comment_start {
            if line.contains(start_marker) {
                return vec![Token {
                    text: line.to_string(),
                    token_type: TokenType::Comment,
                }];
            }
        }

        self.tokenize_code(line, keywords, types)
    }

    fn tokenize_code(&self, code: &str, keywords: &[&str], types: &[&str]) -> Vec<Token> {
        let mut tokens = Vec::new();
        let mut current_word = String::new();
        let mut in_string = false;
        let mut string_char = '"';
        let mut chars = code.chars().peekable();

        while let Some(ch) = chars.next() {
            if ch == '"' || ch == '\'' {
                if !in_string {
                    if !current_word.is_empty() {
                        tokens.push(self.classify_word(&current_word, keywords, types));
                        current_word.clear();
                    }
                    in_string = true;
                    string_char = ch;
                    current_word.push(ch);
                } else if ch == string_char {
                    current_word.push(ch);
                    tokens.push(Token {
                        text: current_word.clone(),
                        token_type: TokenType::String,
                    });
                    current_word.clear();
                    in_string = false;
                } else {
                    current_word.push(ch);
                }
                continue;
            }

            if in_string {
                current_word.push(ch);
                continue;
            }

            if ch.is_numeric() && current_word.is_empty() {
                current_word.push(ch);
                while let Some(&next) = chars.peek() {
                    if next.is_numeric() || next == '.' || next == 'x' || next == 'b' || next == 'o' {
                        current_word.push(chars.next().unwrap());
                    } else {
                        break;
                    }
                }
                tokens.push(Token {
                    text: current_word.clone(),
                    token_type: TokenType::Number,
                });
                current_word.clear();
                continue;
            }

            if "+-*/%=<>!&|^~(){}[];:,.".contains(ch) {
                if !current_word.is_empty() {
                    tokens.push(self.classify_word(&current_word, keywords, types));
                    current_word.clear();
                }
                tokens.push(Token {
                    text: ch.to_string(),
                    token_type: if "+-*/%=<>!&|^~".contains(ch) {
                        TokenType::Operator
                    } else {
                        TokenType::Punctuation
                    },
                });
                continue;
            }

            if ch.is_whitespace() {
                if !current_word.is_empty() {
                    tokens.push(self.classify_word(&current_word, keywords, types));
                    current_word.clear();
                }
                tokens.push(Token {
                    text: ch.to_string(),
                    token_type: TokenType::Normal,
                });
                continue;
            }

            current_word.push(ch);
        }

        if !current_word.is_empty() {
            tokens.push(if in_string {
                Token {
                    text: current_word,
                    token_type: TokenType::String,
                }
            } else {
                self.classify_word(&current_word, keywords, types)
            });
        }

        if tokens.is_empty() {
            tokens.push(Token {
                text: code.to_string(),
                token_type: TokenType::Normal,
            });
        }

        tokens
    }

    fn classify_word(&self, word: &str, keywords: &[&str], types: &[&str]) -> Token {
        let token_type = if keywords.contains(&word) {
            TokenType::Keyword
        } else if types.contains(&word) {
            TokenType::Type
        } else if word.chars().all(|c| c.is_uppercase() || c == '_') && word.len() > 1 {
            TokenType::Constant
        } else if word.starts_with('#') || word.starts_with('@') {
            TokenType::Attribute
        } else if word.ends_with('!') {
            TokenType::Macro
        } else {
            TokenType::Variable
        };

        Token {
            text: word.to_string(),
            token_type,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rust_highlighting() {
        let highlighter = SyntaxHighlighter::new(Some("Rust".to_string()));
        let tokens = highlighter.highlight_line("fn main() {");
        assert_eq!(tokens[0].token_type, TokenType::Keyword);
    }

    #[test]
    fn test_comment_detection() {
        let highlighter = SyntaxHighlighter::new(Some("Rust".to_string()));
        let tokens = highlighter.highlight_line("// This is a comment");
        assert_eq!(tokens[0].token_type, TokenType::Comment);
    }
}