extern crate regex;

use regex::Regex;

use lexer::Lexem;
use lexer::next_lexem;

pub struct Parser {
    lexems: Vec<Lexem>,
    index: usize,
}

impl Parser {
    pub fn new() -> Parser {
        Parser {
            lexems: vec![],
            index: 0
        }
    }

    pub fn parse<'a>(&mut self, query: &String) -> Result<Query, &'a str> {
        let mut i = 0;
        loop {
            match next_lexem(&query, i) {
                Ok((lexem, skip)) => {
                    i = skip;
                    self.lexems.push(lexem);
                },
                Err(_) => break
            }
        }

        let fields = self.parse_fields();
        let root = self.parse_root();
        let expr = self.parse_where();

        Ok(Query {
            fields,
            root,
            expr,
        })
    }

    fn parse_fields(&mut self) -> Vec<String> {
        let mut fields = vec![];
        let mut skip = 0;

        let lexems = &self.lexems;
        for lexem in lexems {
            match lexem {
                &Lexem::Field(ref s) => {
                    let mut ss = String::new();
                    ss.push_str(s);
                    fields.push(ss);

                    skip += 1;
                },
                &Lexem::Comma => {
                    skip += 1;
                },
                _ => break
            }
        }

        self.index = skip;

        fields
    }

    fn parse_root(&mut self) -> String {
        let root;
        let first = self.lexems.get(0 + self.index).unwrap();

        match first {
            &Lexem::From => {
                let second = self.lexems.get(1 + self.index).unwrap();

                match second {
                    &Lexem::String(ref s) | &Lexem::Field(ref s) => {
                        let mut ss = String::new();
                        ss.push_str(s);
                        root = ss;
                    },
                    _ => {
                        panic!("Error parsing directory to search in");
                    }
                }
            },
            _ => {
                panic!("Error parsing directory to search in");
            }
        };

        self.index += 2;

        root
    }

    fn parse_where(&mut self) -> Option<Box<Expr>> {
        let lexem = self.next_lexem();

        match lexem {
            Some(Lexem::Where) => {
                self.parse_or()
            },
            _ => None
        }
    }

    fn parse_or(&mut self) -> Option<Box<Expr>> {
        let mut node = self.parse_and();

        loop {
            let lexem = self.next_lexem();
            if let Some(Lexem::Or) = lexem {
                node = Some(Box::new(Expr::node(node, Some(LogicalOp::Or), self.parse_and())));
            } else {
                self.rollback_lexem();
                break;
            }
        }

        node
    }

    fn parse_and(&mut self) -> Option<Box<Expr>> {
        let mut node = self.parse_cond();

        loop {
            let lexem = self.next_lexem();
            if let Some(Lexem::And) = lexem {
                node = Some(Box::new(Expr::node(node, Some(LogicalOp::And), self.parse_cond())));
            } else {
                self.rollback_lexem();
                break;
            }
        }

        node
    }

    fn parse_cond(&mut self) -> Option<Box<Expr>> {
        let lexem = self.next_lexem();

        match lexem {
            Some(Lexem::Field(ref s)) => {

                let lexem2 = self.next_lexem();

                if let Some(Lexem::Operator(ref s2)) = lexem2 {

                    let lexem3 = self.next_lexem();

                    match lexem3 {
                        Some(Lexem::String(ref s3)) | Some(Lexem::Field(ref s3)) => {
                            let mut ss = String::new();
                            ss.push_str(s);

                            let mut ss2 = String::new();
                            ss2.push_str(s2);

                            let mut ss3 = String::new();
                            ss3.push_str(s3);

                            let expr = match is_glob(s3) {
                                true => {
                                    let pattern = convert_glob_to_pattern(s3);
                                    let regex = Regex::new(&pattern).unwrap();

                                    Expr::leaf_regex(ss, Op::from(ss2), ss3, regex)
                                },
                                false => Expr::leaf(ss, Op::from(ss2), ss3)
                            };

                            Some(Box::new(expr))
                        },
                        _ => None
                    }
                } else {
                    None
                }
            },
            Some(Lexem::Open) => {
                let expr = self.parse_or();
                let lexem4 = self.next_lexem();

                match lexem4 {
                    Some(Lexem::Close) => expr,
                    _ => None
                }
            },
            _ => None
        }
    }

    fn next_lexem(&mut self) -> Option<Lexem> {
        let lexem = self.lexems.get(self.index );
        self.index += 1;

        match lexem {
            Some(lexem) => Some(lexem.clone()),
            None => None
        }
    }

    fn rollback_lexem(&mut self) {
        self.index -= 1;
    }
}

fn is_glob(s: &str) -> bool {
    s.contains('.') || s.contains('*') || s.contains('?')
}

fn convert_glob_to_pattern(s: &str) -> String {
    let string = s.to_string();

    string
        .replace(".", "\\.")
        .replace("*", ".*")
        .replace("?", ".{1}")
        .replace("[", "\\[")
        .replace("]", "\\]")
        .replace("^", "\\^")
        .replace("$", "\\$");

    format!("^{}$", string)
}

#[derive(Debug)]
pub struct Query {
    pub fields: Vec<String>,
    pub root: String,
    pub expr: Option<Box<Expr>>
}

#[derive(Debug)]
pub struct Expr {
    pub left: Option<Box<Expr>>,
    pub logical_op: Option<LogicalOp>,
    pub right: Option<Box<Expr>>,

    pub field: Option<String>,
    pub op: Option<Op>,
    pub val: Option<String>,
    pub regex: Option<Regex>,
}

impl Expr {
    fn node(left: Option<Box<Expr>>, logical_op: Option<LogicalOp>, right: Option<Box<Expr>>) -> Expr {
        Expr {
            left,
            logical_op,
            right,

            field: None,
            op: None,
            val: None,
            regex: None,
        }
    }

    fn leaf(field: String, op: Option<Op>, val: String) -> Expr {
        Expr {
            left: None,
            logical_op: None,
            right: None,

            field: Some(field),
            op,
            val: Some(val),
            regex: None,
        }
    }

    fn leaf_regex(field: String, op: Option<Op>, val: String, regex: Regex) -> Expr {
        Expr {
            left: None,
            logical_op: None,
            right: None,

            field: Some(field),
            op,
            val: Some(val),
            regex: Some(regex),
        }
    }
}

#[derive(Debug)]
pub enum Op {
    Eq,
    Ne,
    Gt,
    Gte,
    Lt,
    Lte,
}

impl Op {
    fn from(text: String) -> Option<Op> {
        if text.eq_ignore_ascii_case("=") {
            return Some(Op::Eq);
        } else if text.eq_ignore_ascii_case("==") {
            return  Some(Op::Eq);
        } else if text.eq_ignore_ascii_case("eq") {
            return  Some(Op::Eq);
        } else if text.eq_ignore_ascii_case("!=") {
            return  Some(Op::Ne);
        } else if text.eq_ignore_ascii_case("<>") {
            return  Some(Op::Ne);
        } else if text.eq_ignore_ascii_case("ne") {
            return  Some(Op::Ne);
        } else if text.eq_ignore_ascii_case(">") {
            return  Some(Op::Gt);
        } else if text.eq_ignore_ascii_case("gt") {
            return  Some(Op::Gt);
        } else if text.eq_ignore_ascii_case(">=") {
            return  Some(Op::Gte);
        } else if text.eq_ignore_ascii_case("gte") {
            return  Some(Op::Gte);
        } else if text.eq_ignore_ascii_case("<") {
            return  Some(Op::Lt);
        } else if text.eq_ignore_ascii_case("lt") {
            return  Some(Op::Lt);
        } else if text.eq_ignore_ascii_case("<=") {
            return  Some(Op::Lte);
        } else if text.eq_ignore_ascii_case("lte") {
            return  Some(Op::Lte);
        }

        None
    }
}

#[derive(Debug)]
pub enum LogicalOp {
    And,
    Or,
}