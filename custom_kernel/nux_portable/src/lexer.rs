#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Print,
    Println,
    Input,
    Class,
    Func,
    Var,
    Return,
    New,
    If,
    Else,
    While,
    For,
    Asm,
    Identifier(String),
    String(String),

    Number(i64),
    LParen,
    RParen,
    LBrace,
    RBrace,
    Slash,
    Star,
    Percent,
    Eq,
    EqEq,
    NotEq,
    Lt,
    Gt,
    LtEq,
    GtEq,
    And,
    Or,
    SemiColon,
    Dot,
    Comma,
    Plus,
    Minus,
    EOF,
}

#[derive(Debug, Clone, Copy)]
pub struct Span {
    pub line: usize,
    pub col: usize,
}

pub struct Lexer {
    input: Vec<char>,
    pos: usize,
    line: usize,
    col: usize,
}

impl Lexer {
    pub fn new(input: &str) -> Self {
        Self {
            input: input.chars().collect(),
            pos: 0,
            line: 1,
            col: 1,
        }
    }

    pub fn next_token(&mut self) -> (Token, Span) {
        self.skip_whitespace();
        let start_span = Span { line: self.line, col: self.col };
        
        if self.pos >= self.input.len() {
            return (Token::EOF, start_span);
        }
        
        let c = self.input[self.pos];
        
        // Helper to advance and track pos/col
        // But wait, skip_whitespace advances too.
        // We need centralized "advance_char" method to track line/col correctly.
        
        // Let's refactor slightly to just peek here and let specific handlers consume.
        // Current implementation uses self.pos manually.
        // I will stick to existing style but update line/col manually.
        
        match c {
            '+' => { self.advance_pos(); (Token::Plus, start_span) },
            '-' => { self.advance_pos(); (Token::Minus, start_span) },
            '*' => { self.advance_pos(); (Token::Star, start_span) },
            '%' => { self.advance_pos(); (Token::Percent, start_span) },
            '(' => { self.advance_pos(); (Token::LParen, start_span) },
            ')' => { self.advance_pos(); (Token::RParen, start_span) },
            '{' => { self.advance_pos(); (Token::LBrace, start_span) },
            '}' => { self.advance_pos(); (Token::RBrace, start_span) },
            ';' => { self.advance_pos(); (Token::SemiColon, start_span) },
            '.' => { self.advance_pos(); (Token::Dot, start_span) },
            ',' => { self.advance_pos(); (Token::Comma, start_span) },
            '/' => {
                // Check if comment
                if self.pos + 1 < self.input.len() && self.input[self.pos + 1] == '/' {
                     // Comment
                     while self.pos < self.input.len() && self.input[self.pos] != '\n' {
                         self.advance_pos();
                     }
                     self.next_token() // Recursively get next token, but wait, self.line needs update?
                     // My advance_pos will handle newline?
                } else {
                    self.advance_pos();
                    (Token::Slash, start_span)
                }
            },
            '=' | '!' | '<' | '>' | '&' | '|' => {
                 self.lex_operator(start_span)
            },
            '"' => self.lex_string(start_span),
            _ if c.is_digit(10) => self.lex_number(start_span),
            _ if c.is_alphabetic() || c == '_' => self.lex_identifier(start_span),
            _ => {
                self.advance_pos(); // Skip unknown
                (Token::Identifier(format!("UNKNOWN_CHAR_{}", c)), start_span)
            }
        }
    }
    
    fn advance_pos(&mut self) {
        if self.pos < self.input.len() {
            let c = self.input[self.pos];
            self.pos += 1;
            if c == '\n' {
                self.line += 1;
                self.col = 1;
            } else {
                self.col += 1;
            }
        }
    }

    fn skip_whitespace(&mut self) {
        while self.pos < self.input.len() && self.input[self.pos].is_whitespace() {
            self.advance_pos();
        }
    }


    fn lex_number(&mut self, start_span: Span) -> (Token, Span) {
        let mut s = String::new();
        while self.pos < self.input.len() && self.input[self.pos].is_digit(10) {
            s.push(self.input[self.pos]);
            self.advance_pos();
        }
        (Token::Number(s.parse().unwrap_or(0)), start_span)
    }

    fn lex_identifier(&mut self, start_span: Span) -> (Token, Span) {
        let mut text = String::new();
        while self.pos < self.input.len() && (self.input[self.pos].is_alphanumeric() || self.input[self.pos] == '_') {
            text.push(self.input[self.pos]);
            self.advance_pos();
        }

        let token = match text.as_str() {
            "print" => Token::Print,
            "println" => Token::Println,
            "input" => Token::Input,
            "func" => Token::Func,
            "var" => Token::Var,
            "return" => Token::Return,
            "new" => Token::New,
            "if" => Token::If,
            "else" => Token::Else,
            "while" => Token::While,
            "for" => Token::For,
            "asm" => Token::Asm,
            _ => Token::Identifier(text),
        };
        (token, start_span)
    }

    fn lex_string(&mut self, start_span: Span) -> (Token, Span) {
        self.advance_pos(); // Skip quote
        let mut s = String::new();
        while self.pos < self.input.len() && self.input[self.pos] != '"' {
             s.push(self.input[self.pos]);
             self.advance_pos();
        }
        self.advance_pos(); // Skip closing quote
        (Token::String(s), start_span)
    }
    
    fn lex_operator(&mut self, start_span: Span) -> (Token, Span) {
        let c = self.input[self.pos];
        self.advance_pos();
        
        if self.pos < self.input.len() {
            let next = self.input[self.pos];
            if c == '=' && next == '=' { self.advance_pos(); return (Token::EqEq, start_span); }
            if c == '!' && next == '=' { self.advance_pos(); return (Token::NotEq, start_span); }
            if c == '<' && next == '=' { self.advance_pos(); return (Token::LtEq, start_span); }
            if c == '>' && next == '=' { self.advance_pos(); return (Token::GtEq, start_span); }
            if c == '&' && next == '&' { self.advance_pos(); return (Token::And, start_span); }
            if c == '|' && next == '|' { self.advance_pos(); return (Token::Or, start_span); }
        }
        
        match c {
            '=' => (Token::Eq, start_span),
            '<' => (Token::Lt, start_span),
            '>' => (Token::Gt, start_span),
            _ => (Token::Identifier(format!("{}", c)), start_span), // Should not happen often
        }
    }
}
