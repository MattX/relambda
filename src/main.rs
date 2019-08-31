use std::iter::Peekable;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum Combinator {
    K,
    S,
    I,
    Dot(char),
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct Application {
    func: Box<SyntaxTree>,
    arg: Box<SyntaxTree>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
enum SyntaxTree {
    Combinator(Combinator),
    Application(Application),
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
struct CharPos {
    item: char,
    position: (usize, usize),
}

fn consume_whitespace<I: Iterator<Item = CharPos>>(iterator: &mut Peekable<I>) {
    while iterator
        .peek()
        .map(|c| c.item.is_whitespace())
        .unwrap_or(false)
    {
        iterator.next().unwrap();
    }
}

fn parse<I: Iterator<Item = CharPos>>(iterator: &mut Peekable<I>) -> Result<SyntaxTree, String> {
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
        '`' => parse(iterator).and_then(|func| {
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

struct CharPosIterator<I: Iterator<Item = char>> {
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

fn main() {
    println!(
        "res: {:?}",
        parse(&mut CharPosIterator::new("  ` `ki `k`.💖s  ".chars()).peekable())
    );
}
