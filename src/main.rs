#![deny(clippy::all)]
#![warn(clippy::pedantic)]

use pico_args::Arguments;

use std::{
    env,
    error::Error as StdError,
    fs::File,
    io::{self, stdin, BufRead, BufReader},
    path::PathBuf,
};

#[derive(Debug, Default)]
struct Info {
    nb_bytes: usize,
    nb_lines: usize,
    nb_words: usize,
    nb_chars: usize,
}

fn gather_info(r: impl BufRead) -> io::Result<Info> {
    let mut output = Info::default();

    let mut last = None::<u8>;

    let mut remaining_bytes_in_char = 0u8;
    for b in r.bytes() {
        let b = b?;
        output.nb_bytes += 1;
        if b == b'\n' {
            output.nb_lines += 1;
        }

        match (!b.is_ascii_whitespace(), last) {
            (true, None) => output.nb_words += 1,
            (true, Some(x)) if x.is_ascii_whitespace() => output.nb_words += 1,
            _ => {}
        }
        last = Some(b);

        if remaining_bytes_in_char == 0 {
            if b >> 4 == 0b00001111 {
                remaining_bytes_in_char = 4;
            } else if b >> 5 == 0b00000111 {
                remaining_bytes_in_char = 3;
            } else if b >> 6 == 0b00000011 {
                remaining_bytes_in_char = 2;
            } else {
                remaining_bytes_in_char = 1;
            }
            output.nb_chars += 1;
        }
        remaining_bytes_in_char -= 1;
    }

    Ok(output)
}

fn main() -> Result<(), Box<dyn StdError>> {
    let mut args = Arguments::from_env();

    let mut count_bytes = args.contains("-c");
    let mut count_lines = args.contains("-l");
    let mut count_words = args.contains("-w");
    let count_multibyte = args.contains("-m");

    if !(count_bytes || count_lines || count_words || count_multibyte) {
        count_bytes = true;
        count_lines = true;
        count_words = true;
    }

    if count_multibyte && !matches!(env::var("LC_CTYPE").as_deref(), Ok("UTF-8")) {
        count_bytes = true;
    }

    let (
        Info {
            nb_bytes,
            nb_lines,
            nb_words,
            nb_chars,
        },
        path,
    ) = if let Some(path) = args.opt_free_from_str::<PathBuf>()? {
        let f = File::open(&path)?;
        (gather_info(BufReader::new(f))?, Some(path))
    } else {
        (gather_info(BufReader::new(stdin()))?, None)
    };

    if count_lines {
        print!("   {nb_lines}");
    }

    if count_words {
        print!("   {nb_words}");
    }

    if count_bytes {
        print!("   {nb_bytes}");
    } else if count_multibyte {
        print!("   {nb_chars}");
    }

    if let Some(path) = path {
        println!(" {}", path.display());
    } else {
        println!()
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;
    #[test]
    fn is_line_break_ascii_whitespace() {
        assert!(b'\n'.is_ascii_whitespace())
    }

    #[test]
    fn nb_words_simple() {
        let cursor = Cursor::new(include_bytes!("../ref_wc.txt"));
        let out = gather_info(cursor).unwrap();
        assert_eq!(out.nb_words, 4)
    }

    #[test]
    fn nb_words_real() {
        let cursor = Cursor::new(include_bytes!("../test.txt"));
        let out = gather_info(cursor).unwrap();
        assert_eq!(out.nb_words, 58164);
        assert_eq!(out.nb_bytes, 342190);
        assert_eq!(out.nb_lines, 7145);
        assert_eq!(out.nb_chars, 339292);
    }
}
