use std::fmt;

/// mi mathvariant attribute
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MathVariant {
    Default,
    Normal,
}

impl fmt::Display for MathVariant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MathVariant::Default => write!(f, ""),
            MathVariant::Normal => write!(f, r#" mathvariant="normal""#),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Accent {
    True,
    False,
}

impl fmt::Display for Accent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Accent::True => write!(f, "true"),
            Accent::False => write!(f, "false"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LineThickness {
    Thin,
    Medium,
    Thick,
    Length(u8),
}
impl fmt::Display for LineThickness {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LineThickness::Thin => write!(f, r#" linethickness="thin""#),
            LineThickness::Medium => write!(f, r#""#),
            LineThickness::Thick => write!(f, r#" linethickness="medium""#),
            LineThickness::Length(l) => write!(f, r#" linethickness="{}""#, l),
        }
    }
}

/// display style
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DisplayStyle {
    True = 1,
    False = 2,
}

impl fmt::Display for DisplayStyle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DisplayStyle::True => write!(f, r#" displaystyle="true""#),
            DisplayStyle::False => write!(f, r#" displaystyle="false""#),
        }
    }
}

/// mi mathvariant attribute
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TextTransform {
    Bold,
    BoldFraktur,
    BoldItalic,
    BoldSansSerif,
    BoldScript,
    DoubleStruck,
    Fraktur,
    Initial,
    Italic,
    Looped,
    Monospace,
    SansSerif,
    SansSerifBoldItalic,
    SansSerifItalic,
    Script,
    Stretched,
    Tailed,
}

struct SingleTransform {
    begin_range: char,
    end_range: char,
    offset: u32,
}

impl SingleTransform {
    fn new(begin: char, end: char, offset: u32) -> Self {
        SingleTransform {
            begin_range: begin,
            end_range: end,
            offset,
        }
    }
}

struct Transforms(Vec<SingleTransform>);

impl Transforms {
    fn new(tforms: Vec<(char, char, u32)>) -> Self {
        Transforms(
            tforms
                .into_iter()
                .map(|(b, e, o)| SingleTransform::new(b, e, o))
                .collect(),
        )
    }
    fn transform(&self, c: char) -> char {
        for tform in &self.0 {
            if c >= tform.begin_range && c <= tform.end_range {
                return std::char::from_u32(c as u32 + tform.offset).unwrap();
            }
        }
        c
    }
}

impl TextTransform {
    fn transform(&self, c: char) -> char {
        match self {
            TextTransform::BoldScript => {
                Transforms::new(vec![('A', 'Z', 0x1d48f), ('a', 'z', 0x1d489)]).transform(c)
            }
        }
    }
}
