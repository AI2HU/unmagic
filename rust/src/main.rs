use std::env;
use std::fs::File;
use std::io::{self, Write};
use std::process;

use memchr::memchr;
use memmap2::{Advice, Mmap, UncheckedAdvice};

// Single pass over the mmapped bytes: numbers are parsed while scanning for their
// delimiter, text fields are only skipped, and nothing is allocated per row.

struct Totals {
    rows: u64,
    salary_cents: i64,
    age: u64,
    active: u64,
    score_cents: i64,
    last_id: u64,
    last_name: (usize, usize),
    last_email: (usize, usize),
    last_city: (usize, usize),
}

#[inline(always)]
fn digit(b: &[u8], i: usize) -> Option<u64> {
    let d = b.get(i)?.wrapping_sub(b'0');
    (d < 10).then_some(d as u64)
}

// Unsigned integer starting at `i`; returns (value, index of the first non-digit).
#[inline(always)]
fn uint(b: &[u8], mut i: usize) -> Option<(u64, usize)> {
    let mut v = digit(b, i)?;
    i += 1;
    while let Some(d) = digit(b, i) {
        v = v * 10 + d;
        i += 1;
    }
    Some((v, i))
}

// "123" or "123.4" or "123.45" as hundredths.
#[inline(always)]
fn cents(b: &[u8], i: usize) -> Option<(i64, usize)> {
    let (whole, mut i) = uint(b, i)?;
    let mut frac = 0;
    if b.get(i) == Some(&b'.') {
        frac = digit(b, i + 1)? * 10;
        i += 2;
        if let Some(d) = digit(b, i) {
            frac += d;
            i += 1;
        }
    }
    Some(((whole * 100 + frac) as i64, i))
}

#[inline(always)]
fn expect(b: &[u8], i: usize, c: u8) -> Option<usize> {
    (b.get(i) == Some(&c)).then_some(i + 1)
}

// Text field starting at `i`; returns its span and the index just past the comma.
#[inline(always)]
fn text(b: &[u8], i: usize) -> Option<((usize, usize), usize)> {
    let len = memchr(b',', b.get(i..)?)?;
    Some(((i, i + len), i + len + 1))
}

// Parses one row starting at `i`; returns the index of the next row.
#[inline(always)]
fn row(b: &[u8], i: usize, t: &mut Totals) -> Option<usize> {
    let (id, i) = uint(b, i)?;
    let i = expect(b, i, b',')?;
    let (name, i) = text(b, i)?;
    let (email, i) = text(b, i)?;
    let (age, i) = uint(b, i)?;
    let i = expect(b, i, b',')?;
    let (salary, i) = cents(b, i)?;
    let i = expect(b, i, b',')?;
    let (city, i) = text(b, i)?;
    let (active, i) = if b.get(i..i + 5) == Some(b"true,") {
        (1, i + 5)
    } else if b.get(i..i + 6) == Some(b"false,") {
        (0, i + 6)
    } else {
        return None;
    };
    let (score, mut i) = cents(b, i)?;
    if b.get(i) == Some(&b'\r') {
        i += 1;
    }
    match b.get(i) {
        Some(b'\n') => i += 1,
        None => {}
        Some(_) => return None,
    }

    t.rows += 1;
    t.salary_cents += salary;
    t.age += age;
    t.active += active;
    t.score_cents += score;
    t.last_id = id;
    t.last_name = name;
    t.last_email = email;
    t.last_city = city;
    Some(i)
}

fn fail(msg: impl std::fmt::Display) -> ! {
    eprintln!("{msg}");
    process::exit(1);
}

fn main() {
    let path = env::args().nth(1).unwrap_or_else(|| "data.csv".to_string());
    let file = File::open(&path).unwrap_or_else(|e| fail(format_args!("cannot read {path}: {e}")));
    // SAFETY: the file is only read; it must not be truncated by another process while mapped.
    let map = unsafe { Mmap::map(&file) }.unwrap_or_else(|e| fail(format_args!("cannot map {path}: {e}")));
    let _ = map.advise(Advice::Sequential);
    let b: &[u8] = &map;

    let mut t = Totals {
        rows: 0,
        salary_cents: 0,
        age: 0,
        active: 0,
        score_cents: 0,
        last_id: 0,
        last_name: (0, 0),
        last_email: (0, 0),
        last_city: (0, 0),
    };

    // Pages behind the cursor are never read again; dropping them keeps RSS flat.
    const RELEASE: usize = 1 << 20;
    let mut released = 0;
    let mut i = memchr(b'\n', b).map_or(b.len(), |n| n + 1);
    while i < b.len() {
        if i - released >= 2 * RELEASE {
            let upto = i & !(RELEASE - 1);
            // SAFETY: read-only file mapping; [released, upto) is never touched again.
            let _ = unsafe { map.unchecked_advise_range(UncheckedAdvice::DontNeed, released, upto - released) };
            released = upto;
        }
        match b[i] {
            b'\n' => i += 1,
            b'\r' if b.get(i + 1) == Some(&b'\n') => i += 2,
            _ => {
                i = row(b, i, &mut t)
                    .unwrap_or_else(|| fail(format_args!("parse error on data row {}", t.rows + 1)))
            }
        }
    }

    let rows = t.rows as f64;
    let s = |(a, z): (usize, usize)| String::from_utf8_lossy(&b[a..z]);
    let mut out = io::stdout().lock();
    let _ = writeln!(out, "rows         : {}", t.rows);
    let _ = writeln!(out, "total salary : {}.{:02}", t.salary_cents / 100, t.salary_cents % 100);
    let _ = writeln!(out, "mean age     : {:.4}", t.age as f64 / rows);
    let _ = writeln!(out, "active       : {}", t.active);
    let _ = writeln!(out, "mean score   : {:.4}", t.score_cents as f64 / 100.0 / rows);
    if t.rows > 0 {
        let _ = writeln!(
            out,
            "last record  : #{} {} <{}> {}",
            t.last_id,
            s(t.last_name),
            s(t.last_email),
            s(t.last_city)
        );
    }
}
