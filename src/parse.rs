use std::iter::Peekable;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Combinator {
    I,
    K,
    S,
    Dot(char),
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Application {
    pub func: Box<SyntaxTree>,
    pub arg: Box<SyntaxTree>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum SyntaxTree {
    Combinator(Combinator),
    Application(Application),
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct CharPos {
    pub item: char,
    pub position: (usize, usize),
}

fn read_to_newline<I: Iterator<Item = CharPos>>(iterator: &mut Peekable<I>) {
    while let Some(cp) = iterator.next() {
        if cp.item == '\n' {
            return;
        }
    }
}

fn consume_whitespace<I: Iterator<Item = CharPos>>(iterator: &mut Peekable<I>) {
    loop {
        match iterator.peek() {
            Some(c) if c.item == '#' => read_to_newline(iterator),
            Some(c) if c.item.is_whitespace() => {
                iterator.next().unwrap();
            }
            _ => break,
        }
    }
}

fn parse<I: Iterator<Item = CharPos>>(
    iterator: &mut Peekable<I>,
) -> Result<SyntaxTree, String> {
    consume_whitespace(iterator);
    let token = iterator
        .next()
        .ok_or_else(|| "unexpected EOF".to_string())?;
    match token.item {
        'k' => Ok(SyntaxTree::Combinator(Combinator::K)),
        's' => Ok(SyntaxTree::Combinator(Combinator::S)),
        'i' => Ok(SyntaxTree::Combinator(Combinator::I)),
        '.' => iterator
            .next()
            .map(|c| SyntaxTree::Combinator(Combinator::Dot(c.item)))
            .ok_or_else(|| format!("unexpected EOF after `.` at {:?}", token.position)),
        'r' => Ok(SyntaxTree::Combinator(Combinator::Dot('\n'))),
        '[' | '`' => parse(iterator).and_then(|func| {
            parse(iterator).map(|arg| {
                SyntaxTree::Application(Application {
                    func: Box::new(func),
                    arg: Box::new(arg),
                })
            })
        }),
        c => Err(format!("unexpected token `{}` at {:?}", c, token.position)),
    }
}

pub fn parse_toplevel<I: Iterator<Item = CharPos>>(
    iterator: &mut Peekable<I>,
) -> Result<SyntaxTree, String> {
    let res = parse(iterator)?;
    consume_whitespace(iterator);
    if let Some(cp) = iterator.next() {
        Err(format!(
            "unexpected character `{}` at {:?}",
            cp.item, cp.position
        ))
    } else {
        Ok(res)
    }
}

pub struct CharPosIterator<I: Iterator<Item = char>> {
    chars: I,
    col: usize,
    line: usize,
    nl: bool,
}

impl<I: Iterator<Item = char>> CharPosIterator<I> {
    pub fn new(chars: I) -> Self {
        Self {
            chars,
            col: 0,
            line: 0,
            nl: false,
        }
    }
}

impl<I: Iterator<Item = char>> Iterator for CharPosIterator<I> {
    type Item = CharPos;

    fn next(&mut self) -> Option<Self::Item> {
        let cur = self.chars.next()?;
        if self.nl {
            self.col = 0;
            self.line += 1;
            self.nl = false;
        }
        self.nl = cur == '\n';
        let cp = CharPos {
            item: cur,
            position: (self.line, self.col),
        };
        self.col += 1;
        Some(cp)
    }
}

fn write_combinator(c: Combinator) -> String {
    match c {
        Combinator::I => "I".into(),
        Combinator::K => "K".into(),
        Combinator::S => "S".into(),
        Combinator::Dot(ch) => format!(".{}", ch),
    }
}

pub fn print_parenthesized(st: &SyntaxTree, depth: usize, direct_depth: usize, into: &mut String) {
    match st {
        SyntaxTree::Combinator(c) => into.extend(write_combinator(*c).chars()),
        SyntaxTree::Application(Application { func, arg }) => {
            into.push('(');
            print_parenthesized(func, depth+1, direct_depth+1, into);
            print_parenthesized(arg, depth+1, 0, into);
            into.push(')');
        }
    }
}
