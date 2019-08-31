#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum Combinator {
    K, S, I, Dot(char)
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct Application {
    func: Box<Element>,
    arg: Box<Element>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
enum Element {
    Combinator(Combinator),
    Application(Application),
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
struct CharPos {
    item: char,
    position: (usize, usize),
}

fn parse<I: Iterator<Item=CharPos>>(iterator: &mut I) -> Result<Element, String> {
    let token = iterator.next().ok_or_else(|| "unexpected EOF".to_string())?;
    match token.item {
        'k' => Ok(Element::Combinator(Combinator::K)),
        's' => Ok(Element::Combinator(Combinator::S)),
        'i' => Ok(Element::Combinator(Combinator::I)),
        '.' => iterator.next().map(|c| Element::Combinator(Combinator::Dot(c.item)))
            .ok_or_else(|| format!("unexpected EOF after `.` at {:?}", token.position)),
        c => Err(format!("unexpected token `{}` at {:?}", c, token.position)),
    }
}

struct CharPosIterator<I: Iterator<Item=char>> {
    chars: I,
    col: usize,
    line: usize,
    nl: bool,
}

impl<I: Iterator<Item=char>> CharPosIterator<I> {
    pub fn new(chars: I) -> Self {
        Self {
            chars,
            col: 0,
            line: 0,
            nl: false,
        }
    }
}

impl<I: Iterator<Item=char>> Iterator for CharPosIterator<I> {
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
    println!("res: {:?}", parse(&mut CharPosIterator::new("k".chars())));
}
