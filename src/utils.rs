use std::cmp::Ord;
use std::{fmt, num::ParseIntError};

use lambda_calculus::Term;
use std::fs::OpenOptions;
use std::io::{self, BufRead, BufReader, Write};

// This was shamelessly stolen from
// https://play.rust-lang.org/?version=stable&mode=debug&edition=2015&gist=e241493d100ecaadac3c99f37d0f766f
pub fn decode_hex(s: &str) -> Result<Vec<u8>, DecodeHexError> {
    if s.len() & 1 == 1 {
        Err(DecodeHexError::OddLength)
    } else {
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|e| e.into()))
            .collect()
    }
}

const HEX_BYTES: &str = "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f\
                         202122232425262728292a2b2c2d2e2f303132333435363738393a3b3c3d3e3f\
                         404142434445464748494a4b4c4d4e4f505152535455565758595a5b5c5d5e5f\
                         606162636465666768696a6b6c6d6e6f707172737475767778797a7b7c7d7e7f\
                         808182838485868788898a8b8c8d8e8f909192939495969798999a9b9c9d9e9f\
                         a0a1a2a3a4a5a6a7a8a9aaabacadaeafb0b1b2b3b4b5b6b7b8b9babbbcbdbebf\
                         c0c1c2c3c4c5c6c7c8c9cacbcccdcecfd0d1d2d3d4d5d6d7d8d9dadbdcdddedf\
                         e0e1e2e3e4e5e6e7e8e9eaebecedeeeff0f1f2f3f4f5f6f7f8f9fafbfcfdfeff";

pub fn encode_hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|&b| unsafe {
            let i = 2 * b as usize;
            HEX_BYTES.get_unchecked(i..i + 2)
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeHexError {
    OddLength,
    ParseInt(ParseIntError),
}

impl From<ParseIntError> for DecodeHexError {
    fn from(e: ParseIntError) -> Self {
        DecodeHexError::ParseInt(e)
    }
}

impl fmt::Display for DecodeHexError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            DecodeHexError::OddLength => "input string has an odd number of bytes".fmt(f),
            DecodeHexError::ParseInt(e) => e.fmt(f),
        }
    }
}

impl std::error::Error for DecodeHexError {}

// Utility to make a non-ord type temporarily ord for use in priority queues.
pub struct HeapObject<U, T>
where
    U: Ord,
{
    priority: U,
    obj: T,
}

impl<U, T> HeapObject<U, T>
where
    U: Ord + Copy,
    T: Clone,
{
    pub fn new(priority: U, obj: T) -> HeapObject<U, T> {
        HeapObject { priority, obj }
    }

    pub fn to_tuple(&self) -> (U, T) {
        (self.priority, self.obj.clone())
    }
}

impl<U, T> Ord for HeapObject<U, T>
where
    U: Ord,
{
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.priority.cmp(&other.priority)
    }
}

impl<U, T> PartialOrd for HeapObject<U, T>
where
    U: Ord,
{
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<U, T> PartialEq for HeapObject<U, T>
where
    U: Ord,
{
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority
    }
}

impl<U, T> Eq for HeapObject<U, T> where U: Ord {}

/// Read lambda expressions from stdin and return an iterator over them
pub fn read_inputs() -> impl Iterator<Item = Term> {
    let mut expression_strings = Vec::<String>::new();
    let stdin = io::stdin();
    let reader = BufReader::new(stdin.lock());

    for line in reader.lines() {
        match line {
            Ok(line) => expression_strings.push(line),
            Err(_) => break,
        }
    }

    let expressions = expression_strings
        .iter()
        .map(|s| lambda_calculus::parse(s, lambda_calculus::Classic).unwrap())
        .collect::<Vec<Term>>();
    expressions.into_iter()
}

pub fn dump_series_to_file<T>(fname: &str, series: &[T], id: &[usize]) -> io::Result<()>
where
    T: fmt::Debug,
{
    let mut file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(format!("{fname}.txt"))?;
    write!(file, "{id:?}; ")?;
    for i in series {
        write!(file, "{:?}; ", i)?;
    }
    writeln!(file)?;
    Ok(())
}
