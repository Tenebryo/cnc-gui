use pest::Parser;

#[derive(Parser)]
#[grammar = "grammars/gcode.pest"]
pub struct GCodeParser;

pub struct GCodeLine<'i> {
    pub line : &'i str,
    pub words : Box<[(char, f32, u32, u32)]>,
}

impl GCodeLine<'_> {
    pub fn value_for(&self, c : char) -> Option<f32> {
        self.words.iter()
            .find(|v| v.0 == c)
            .map(|i| i.1)
    }

    pub fn mnemonic(&self) -> char {
        self.words.first().unwrap().0
    }

    pub fn major(&self) -> u32 {
        self.words.first().unwrap().2
    }

    pub fn minor(&self) -> u32 {
        self.words.first().unwrap().3
    }
}

pub fn parse<'i>(program : &'i str) -> Vec<GCodeLine<'i>> {

    GCodeParser::parse(Rule::file, program).unwrap()
        .map(|l| {

            let line = l.as_str();

            let mut words = l.into_inner()
                .map(|w| {
                    let letter = w.as_str().chars().next().unwrap().to_ascii_uppercase();
                    let num = w.into_inner().next().unwrap();
                    let value = num.as_str().parse::<f32>().unwrap();
                    let mut parts = num.into_inner();
                    let major = parts.next().unwrap().as_str().parse::<u32>().unwrap();
                    let minor = parts.next().map(|n| n.as_str().parse::<u32>().unwrap()).unwrap_or(0);

                    (letter, value, major, minor)
                })
                .collect::<Box<[_]>>();

            GCodeLine {
                line,
                words,
            }

        })
        .collect::<Vec<_>>()
}
