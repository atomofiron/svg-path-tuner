mod args;
mod coordination;
mod fixed;
mod scale;
use crate::args::Args;
use crate::coordination::Coordination;
use crate::fixed::Fixed;
use crate::scale::Scale;
use clap::Parser;
use std::fs;
use std::io::{self, Write};
use std::process::exit;

const SEPARATOR: char = ' ';
const PATH_DATA: &str = "android:pathData";
const VIEWPORT_WIDTH: &str = "android:viewportWidth";
const VIEWPORT_HEIGHT: &str = "android:viewportHeight";

fn main() {
    let args = Args::parse();
    args.verify();

    if args.files.is_empty() {
        loop {
            work(args.scale, args.target);
        }
    }

    if let Some(target) = args.target {
        for file in &args.files {
            tune_file(file, args.scale, target);
        }
    }
}

fn work(scale: Scale, coordination: Option<Coordination>) {
    print!("input path: ");
    io::stdout().flush().unwrap();

    let mut line = String::new();
    io::stdin().read_line(&mut line).unwrap();
    if line.len() == 1 && line.chars().nth(0).unwrap() == '\n' {
        exit(0);
    }

    let parts = tokenize(line.trim());
    println!("parts: {}", parts.join(", "));
    match coordination {
        Some(Coordination::Relative) => println!("{}", build_path(&parts, true, scale)),
        Some(Coordination::Absolute) => println!("{}", build_path(&parts, false, scale)),
        None => {
            println!("\nrelative: {}", build_path(&parts, true, scale));
            println!("\nabsolute: {}", build_path(&parts, false, scale));
        }
    }
}

fn tune_file(file: &str, scale: Scale, coordination: Coordination) {
    let xml = fs::read_to_string(file).unwrap_or_else(|error| panic!("{file}: {error}"));
    let (xml, paths) = map_attribute(&xml, PATH_DATA, |value| {
        scale_path_data(value, scale, coordination)
    });
    let (xml, widths) = map_attribute(&xml, VIEWPORT_WIDTH, |value| scale_number(value, scale));
    let (xml, heights) = map_attribute(&xml, VIEWPORT_HEIGHT, |value| scale_number(value, scale));
    fs::write(file, xml).unwrap_or_else(|error| panic!("{file}: {error}"));
    println!("{file}: {paths} pathData, {} viewport, scale {scale}", widths + heights);
}

/// Maps every quoted value of `attribute`, keeping the rest of the document untouched.
fn map_attribute(xml: &str, attribute: &str, mut map: impl FnMut(&str) -> String) -> (String, usize) {
    let mut out = String::with_capacity(xml.len());
    let mut rest = xml;
    let mut count = 0;

    while let Some(start) = rest.find(attribute) {
        if let Some(comment) = rest.find("<!--") {
            if comment < start { // never touch an attribute that is commented out
                let end = match rest[comment..].find("-->") {
                    Some(end) => comment + end + 3,
                    None => rest.len(),
                };
                out.push_str(&rest[..end]);
                rest = &rest[end..];
                continue;
            }
        }

        let name_end = start + attribute.len();
        let tail = &rest[name_end..];
        let value_start = trim_ascii_space(tail);
        let value_start = match value_start.as_bytes().first() {
            Some(b'=') => trim_ascii_space(&value_start[1..]),
            _ => {
                out.push_str(&rest[..name_end]);
                rest = tail;
                continue;
            }
        };
        let quote = match value_start.as_bytes().first() {
            Some(quote @ (b'"' | b'\'')) => *quote as char,
            _ => {
                out.push_str(&rest[..name_end]);
                rest = tail;
                continue;
            }
        };
        let value_end = match value_start[1..].find(quote) {
            Some(end) => end,
            None => break,
        };

        out.push_str(&rest[..name_end]);
        out.push_str(&tail[..tail.len() - value_start.len()]);
        out.push(quote);
        out.push_str(&map(&value_start[1..1 + value_end]));
        out.push(quote);
        count += 1;
        rest = &value_start[value_end + 2..];
    }

    out.push_str(rest);
    (out, count)
}

fn trim_ascii_space(value: &str) -> &str {
    value.trim_start_matches(|c: char| c.is_ascii_whitespace())
}

fn scale_path_data(value: &str, scale: Scale, coordination: Coordination) -> String {
    let parts = tokenize(value);
    if parts.is_empty() {
        return value.to_owned();
    }
    build_path(&parts, coordination == Coordination::Relative, scale)
}

fn scale_number(value: &str, scale: Scale) -> String {
    match fixed::parse(value.trim()) {
        Ok(number) => fixed::format(scale.apply(number)),
        Err(_) => value.to_owned(),
    }
}

fn tokenize(input: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut part = String::new();
    let mut exponent = false;

    for c in input.chars() {
        if exponent {
            exponent = false;
            if c == '-' || c == '+' { // 1.05E-7 1.05E+7
                part.push(c);
                continue;
            }
        }
        if c.is_whitespace() || c == ',' {
            push_part(&mut parts, &mut part);
        } else if is_command(c) {
            push_part(&mut parts, &mut part);
            parts.push(c.to_string());
        } else if c == '-' {
            push_part(&mut parts, &mut part);
            part.push('-');
        } else if c == '+' { // +5 is the same number as 5, but it still splits numbers
            push_part(&mut parts, &mut part);
        } else if c.is_ascii_digit() || c == '.' || c == 'e' || c == 'E' {
            if c == '.' && part.contains('.') { // .5.5 is two numbers
                push_part(&mut parts, &mut part);
            }
            part.push(c);
            exponent = c == 'e' || c == 'E';
        }
    }
    push_part(&mut parts, &mut part);
    parts
}

fn push_part(parts: &mut Vec<String>, part: &mut String) {
    if !part.is_empty() {
        parts.push(std::mem::take(part));
    }
}

fn is_command(c: char) -> bool {
    matches!(c, 'm' | 'a' | 'h' | 'v' | 'l' | 'c' | 's' | 'q' | 't' | 'z' | 'M' | 'A' | 'H' | 'V' | 'L' | 'C' | 'S' | 'Q' | 'T' | 'Z')
}

fn build_path(parts: &[String], relative: bool, scale: Scale) -> String {
    let mut out = String::new();
    let mut x: Fixed = 0;
    let mut y: Fixed = 0;
    let mut index = 0;

    while index < parts.len() {
        let command = parts[index].chars().nth(0).unwrap();
        index += 1;
        if !is_command(command) {
            panic!("command {command}, index {}", index - 1);
        }

        let mut letter = command;
        loop {
            if relative {
                out.push(letter.to_ascii_lowercase());
            } else {
                out.push(letter.to_ascii_uppercase());
            }

            let mut to_x = x;
            let mut to_y = y;

            if matches!(letter, 'a' | 'A') {
                let rx = scale.apply(number(&parts[index]));
                write_coordinate(&mut out, rx, false);
                index += 1;
                let ry = scale.apply(number(&parts[index]));
                write_coordinate(&mut out, ry, true);
                index += 1;
                write_part(&mut out, &parts[index], true);
                index += 1;
                write_part(&mut out, &parts[index], true);
                index += 1;
                write_part(&mut out, &parts[index], true);
                index += 1;
            }

            let points = match letter {
                'a' | 'A' => 1,
                'm' | 'l' | 't' | 'h' | 'v' | 'M' | 'L' | 'T' | 'H' | 'V' => 1,
                's' | 'q' | 'S' | 'Q' => 2,
                'c' | 'C' => 3,
                'z' | 'Z' => 0,
                _ => panic!("command {letter}, index {}", index - 1),
            };

            for point in 0..points {
                match letter {
                    'h' => { to_x = x + scale.apply(number(&parts[index])); index += 1; }
                    'v' => { to_y = y + scale.apply(number(&parts[index])); index += 1; }
                    'H' => { to_x = scale.apply(number(&parts[index])); index += 1; }
                    'V' => { to_y = scale.apply(number(&parts[index])); index += 1; }
                    'm' | 'l' | 't' | 'c' | 's' | 'q' | 'a' => {
                        to_x = x + scale.apply(number(&parts[index])); index += 1;
                        to_y = y + scale.apply(number(&parts[index])); index += 1;
                    }
                    'M' | 'L' | 'T' | 'C' | 'S' | 'Q' | 'A' => {
                        to_x = scale.apply(number(&parts[index])); index += 1;
                        to_y = scale.apply(number(&parts[index])); index += 1;
                    }
                    _ => panic!(),
                }

                let hv = matches!(letter, 'h' | 'v' | 'H' | 'V');
                let a = point > 0 || matches!(letter, 'a' | 'A');
                if "mahlcsqtMAHLCSQT".contains(letter) {
                    write_coordinate(&mut out, if relative { to_x - x } else { to_x }, a);
                }
                if "mavlcsqtMAVLCSQT".contains(letter) {
                    write_coordinate(&mut out, if relative { to_y - y } else { to_y }, !hv);
                }
            }
            x = to_x;
            y = to_y;

            // a command reuses its letter for the implicit parameter sets, moveto turns into lineto
            if points == 0 || index == parts.len() || is_command(parts[index].chars().nth(0).unwrap()) {
                break;
            }
            letter = match letter {
                'm' => 'l',
                'M' => 'L',
                _ => letter,
            };
        }
    }
    out
}

fn number(token: &str) -> Fixed {
    fixed::parse(token).unwrap_or_else(|error| panic!("{error}"))
}

fn write_part(out: &mut String, part: &str, allow_separator: bool) {
    if allow_separator && !part.starts_with('-') {
        out.push(SEPARATOR);
    }
    if part.starts_with('.') || part.starts_with("-.") || !part.ends_with(".0") {
        out.push_str(part);
    } else {
        out.push_str(&part[..part.len() - 2]);
    }
}

fn write_coordinate(out: &mut String, value: Fixed, allow_separator: bool) {
    if allow_separator && value >= 0 {
        out.push(SEPARATOR);
    }
    out.push_str(&fixed::format(value));
}
