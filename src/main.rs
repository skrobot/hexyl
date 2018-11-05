#[macro_use]
extern crate clap;
extern crate ansi_term;

use std::fs::File;
use std::io::{self, prelude::*, StdoutLock};

use clap::{App, AppSettings, Arg};

use ansi_term::Colour;
use ansi_term::Colour::RGB;

const BUFFER_SIZE: usize = 64;

const COLOR_NULL: Colour = RGB(117, 113, 94); // grey
const COLOR_ASCII_PRINTABLE: Colour = RGB(102, 217, 239); // cyan
const COLOR_ASCII_WHITESPACE: Colour = RGB(166, 226, 46); // green
const COLOR_ASCII_OTHER: Colour = RGB(249, 38, 114); // magenta
const COLOR_NONASCII: Colour = RGB(253, 151, 31); // orange

enum ByteCategory {
    Null,
    AsciiPrintable,
    AsciiWhitespace,
    AsciiOther,
    NonAscii,
}

#[derive(Copy, Clone)]
struct Byte(u8);

impl Byte {
    fn category(self) -> ByteCategory {
        if self.0 == 0x00 {
            ByteCategory::Null
        } else if self.0.is_ascii_alphanumeric()
            || self.0.is_ascii_punctuation()
            || self.0.is_ascii_graphic()
        {
            ByteCategory::AsciiPrintable
        } else if self.0.is_ascii_whitespace() {
            ByteCategory::AsciiWhitespace
        } else if self.0.is_ascii() {
            ByteCategory::AsciiOther
        } else {
            ByteCategory::NonAscii
        }
    }

    fn color(&self) -> &'static Colour {
        use ByteCategory::*;

        match self.category() {
            Null => &COLOR_NULL,
            AsciiPrintable => &COLOR_ASCII_PRINTABLE,
            AsciiWhitespace => &COLOR_ASCII_WHITESPACE,
            AsciiOther => &COLOR_ASCII_OTHER,
            NonAscii => &COLOR_NONASCII,
        }
    }

    fn as_char(self) -> char {
        use ByteCategory::*;

        match self.category() {
            Null => '0',
            AsciiPrintable => self.0 as char,
            AsciiWhitespace => '.',
            AsciiOther => '.',
            NonAscii => '.',
        }
    }
}

struct Printer<'a> {
    idx: usize,
    line: Vec<u8>,
    stdout: StdoutLock<'a>,
}

impl<'a> Printer<'a> {
    fn new(stdout: StdoutLock) -> Printer {
        Printer {
            idx: 1,
            line: vec![],
            stdout,
        }
    }

    fn print_byte(&mut self, b: u8) -> io::Result<()> {
        if self.idx % 16 == 1 {
            write!(self.stdout, "  ");
        }

        let byte_str = format!("{:02x} ", b);
        write!(self.stdout, "{}", Byte(b).color().paint(byte_str))?;
        self.line.push(b);

        match self.idx % 16 {
            8 => write!(self.stdout, " ")?,
            0 => {
                self.print_textline()?;
            }
            _ => {}
        }

        self.idx += 1;

        Ok(())
    }

    fn print_textline(&mut self) -> io::Result<()> {
        let fill_spaces = match self.line.len() {
            n if n < 8 => 1 + 3 * (16 - n),
            n => 3 * (16 - n),
        };

        write!(self.stdout, "{}  │", " ".repeat(fill_spaces))?;

        for b in self.line.iter().map(|b| Byte(*b)) {
            let chr = format!("{}", b.as_char());

            write!(self.stdout, "{}", b.color().paint(chr)).ok();
        }

        writeln!(self.stdout, "│");

        self.line.clear();

        Ok(())
    }
}

fn run() -> io::Result<()> {
    let app = App::new(crate_name!())
        .setting(AppSettings::ColorAuto)
        .setting(AppSettings::ColoredHelp)
        .setting(AppSettings::DeriveDisplayOrder)
        .setting(AppSettings::UnifiedHelpMessage)
        .version(crate_version!())
        .arg(Arg::with_name("file").help("to do").required(true));

    let matches = app.get_matches();

    let filename = matches.value_of("file").unwrap();

    let mut buffer = [0; BUFFER_SIZE];
    let mut file = File::open(filename)?;

    let stdout = io::stdout();
    let mut printer = Printer::new(stdout.lock());
    loop {
        let size = file.read(&mut buffer)?;
        if size == 0 {
            break;
        }

        for b in &buffer[..size] {
            let res = printer.print_byte(*b);

            if res.is_err() {
                // Broken pipe
                break;
            }
        }
    }

    // Finish last line
    printer.print_textline().ok();

    Ok(())
}

fn main() {
    let result = run();
    match result {
        Err(err) => {
            eprintln!("Error: {}", err);
            std::process::exit(1);
        }
        Ok(()) => {}
    }
}
