//! A parser for the [Scala](https://www.huygens-fokker.org/scala/) file format.

use nom::{
    bytes::streaming::{tag, take_until},
    character::streaming::{newline, space0},
    multi::count,
    multi::many0,
    number::streaming::float,
    sequence::tuple,
    IResult,
};

fn parse_scala<'a>(scala_text: &'a impl AsRef<str>) -> IResult<&'a str, Scale> {
    let i = scala_text.as_ref();

    let (i, (_comments, description)) = tuple((many_comments, take_line))(i)?;
    let (i, (_comments, _, note_count)) = tuple((many_comments, space0, num_u64))(i)?;
    let (i, notes) = count(tuple((many_comments, note)), note_count as usize)(i)?;

    let notes = notes.into_iter().map(|(_, note)| note).collect();

    let scale = Scale {
        description: description.to_string(),
        notes,
    };

    Ok((i, scale))
}

#[derive(Debug, Clone)]
pub struct Scale {
    pub description: String,
    pub notes: Vec<Note>,
}

impl Scale {
    /// Parse a Scale from a Scala file.
    ///
    /// # Note
    /// Many Scala files found online, specifically in the Scala archive,
    /// are encoded in ISO-8859-1. You will likely need to unsure such cases
    /// are decoded into UTF8 in order to read these files to a string.
    pub fn from_str<'a>(input: &'a impl AsRef<str>) -> Result<Scale, Error> {
        let res = parse_scala(input);

        match res {
            Ok(s) => Ok(s.1),
            Err(e) => Err(Error(e.to_string())),
        }
    }
}

#[derive(Debug)]
pub struct Error(String);

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Error {}

#[derive(Clone, Debug)]
pub enum Note {
    Ratio { numerator: u64, denominator: u64 },
    Cents(f32),
}

fn note(i: &str) -> IResult<&str, Note> {
    let (i, _) = nom::combinator::opt(nom::character::streaming::space0)(i)?;
    nom::branch::alt((note_cents, note_ratio))(i)
}

fn note_cents(i: &str) -> IResult<&str, Note> {
    let (i, f) = float(i)?;

    let (i, _) = take_line(i)?;

    Ok((i, Note::Cents(f)))
}

fn note_ratio(i: &str) -> IResult<&str, Note> {
    let (i, (numerator, _, denominator)) = tuple((num_u64, tag("/"), num_u64))(i)?;

    let note = Note::Ratio {
        numerator,
        denominator,
    };

    Ok((i, note))
}

fn num_u64(i: &str) -> IResult<&str, u64> {
    // A scale probably wont ever have more precision than two u64::MAX
    // and any number below 0 in a ratio or note count is an error in scalas format.
    let (_, number) = nom::character::streaming::u64(i)?;

    Ok((i, number))
}

fn many_comments(i: &str) -> IResult<&str, Vec<&str>> {
    many0(comment)(i)
}

fn comment(i: &str) -> IResult<&str, &str> {
    let (i, _) = tag("!")(i)?;

    let (i, comment) = take_line(i)?;

    Ok((i, comment))
}

fn take_line(i: &str) -> IResult<&str, &str> {
    let (i, line) = take_until("\n")(i)?;
    let (i, _) = nom::combinator::opt(newline)(i)?;

    Ok((i, line.trim()))
}

#[cfg(test)]
mod tests {
    use std::{io::Read, path::PathBuf};

    use crate::parse_scala;

    #[test]
    fn test_all_scl() {
        // to run this test, download the scala file archive
        // and extract the scl folder into the scala-tune folder
        //
        // downloadable here:
        // https://www.huygens-fokker.org/scala/downloads.html#scales
        let dir = std::fs::read_dir("./scl").unwrap();
        let paths: Vec<PathBuf> = dir
            .filter_map(|f| -> Option<PathBuf> {
                let path = f.unwrap().path();
                if let Some(e) = path.extension() {
                    if e == "scl" {
                        return Some(path);
                    }
                }
                None
            })
            .collect();

        for path in paths {
            let mut text = String::new();
            std::io::BufReader::new(std::fs::File::open(path).unwrap())
                .read_to_string(&mut text)
                .unwrap();

            assert!(parse_scala(&text).is_ok());
        }
    }
}
