mod args;
mod coordination;
mod fixed;
mod scale;
mod size;
use crate::args::Args;
use crate::coordination::Coordination;
use crate::fixed::Fixed;
use crate::scale::Scale;
use crate::size::Size;
use clap::Parser;
use std::fs;
use std::io::{self, Write};
use std::ops::Range;
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
            work(args.target);
        }
    }

    if let Some(target) = args.target {
        for file in &args.files {
            tune_file(file, args.size, target);
        }
    }
}

fn work(coordination: Option<Coordination>) {
    print!("input path: ");
    io::stdout().flush().unwrap();

    let mut line = String::new();
    io::stdin().read_line(&mut line).unwrap();
    if line.len() == 1 && line.chars().nth(0).unwrap() == '\n' {
        exit(0);
    }

    let parts = tokenize(line.trim());
    println!("parts: {}", parts.join(", "));
    let scale = Scale::identity();
    match coordination {
        Some(Coordination::Relative) => println!("{}", path_data(&parts, true, scale, "input path")),
        Some(Coordination::Absolute) => println!("{}", path_data(&parts, false, scale, "input path")),
        None => {
            println!("\nrelative: {}", path_data(&parts, true, scale, "input path"));
            println!("\nabsolute: {}", path_data(&parts, false, scale, "input path"));
        }
    }
}

fn tune_file(file: &str, size: Option<Size>, coordination: Coordination) {
    let xml = fs::read_to_string(file).unwrap_or_else(|error| panic!("{file}: {error}"));
    let scale = match Scale::fit(read_viewport(&xml), size) {
        Ok(scale) => scale,
        Err(error) => {
            eprintln!("{file}: {error}, the file was skipped");
            return;
        }
    };
    let (xml, paths) = map_attribute(&xml, PATH_DATA, |value| {
        scale_path_data(value, file, scale, coordination)
    });
    let (xml, viewports) = match size {
        Some(size) => {
            let (xml, widths) = map_attribute(&xml, VIEWPORT_WIDTH, |_| fixed::format(size.width));
            let (xml, heights) = map_attribute(&xml, VIEWPORT_HEIGHT, |_| fixed::format(size.height));
            (xml, widths + heights)
        }
        None => (xml, 0),
    };
    fs::write(file, xml).unwrap_or_else(|error| panic!("{file}: {error}"));
    println!("{file}: {paths} pathData, {viewports} viewport, scale {scale}");
}

/// Reads the viewport of the file, the ratios are computed against it.
fn read_viewport(xml: &str) -> Option<(Fixed, Fixed)> {
    let width = fixed::parse(attribute_value(xml, VIEWPORT_WIDTH)?).ok()?;
    let height = fixed::parse(attribute_value(xml, VIEWPORT_HEIGHT)?).ok()?;
    Some((width, height))
}

fn attribute_value<'a>(xml: &'a str, attribute: &str) -> Option<&'a str> {
    let range = attribute_values(xml, attribute).first().cloned()?;
    Some(&xml[range])
}

/// Byte ranges of every quoted value of `attribute`, values inside XML comments left alone.
fn attribute_values(xml: &str, attribute: &str) -> Vec<Range<usize>> {
    let mut values = Vec::new();
    let mut position = 0;

    while let Some(found) = xml[position..].find(attribute) {
        let name = position + found;
        let name_end = name + attribute.len();

        if let Some(comment) = xml[position..].find("<!--").map(|found| position + found) {
            if comment < name { // an attribute that is commented out is not an attribute
                position = match xml[comment..].find("-->") {
                    Some(end) => comment + end + 3,
                    None => xml.len(),
                };
                continue;
            }
        }

        let tail = &xml[name_end..];
        let open = match tail.find(['"', '\'']) {
            Some(open) => open,
            None => break,
        };
        let between = &tail[..open];
        if !between.contains('=') || !between.chars().all(|c| c.is_ascii_whitespace() || c == '=') {
            position = name_end;
            continue;
        }
        let quote = tail.as_bytes()[open] as char;
        let value = name_end + open + 1;
        let length = match xml[value..].find(quote) {
            Some(length) => length,
            None => break,
        };
        values.push(value..value + length);
        position = value + length + 1;
    }

    values
}

/// Rewrites every quoted value of `attribute`, keeping the rest of the document untouched.
fn map_attribute(xml: &str, attribute: &str, mut map: impl FnMut(&str) -> String) -> (String, usize) {
    let ranges = attribute_values(xml, attribute);
    if ranges.is_empty() {
        return (xml.to_owned(), 0);
    }
    let mut out = String::with_capacity(xml.len());
    let mut position = 0;
    for range in &ranges {
        out.push_str(&xml[position..range.start]);
        out.push_str(&map(&xml[range.clone()]));
        position = range.end;
    }
    out.push_str(&xml[position..]);
    (out, ranges.len())
}

fn scale_path_data(value: &str, file: &str, scale: Scale, coordination: Coordination) -> String {
    let parts = tokenize(value);
    if parts.is_empty() {
        return value.to_owned();
    }
    let path = path_data(&parts, coordination == Coordination::Relative, scale, file);
    if path.is_empty() {
        return value.to_owned(); // nothing scalable in there, the original text stays
    }
    path
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

fn build_path(parts: &[String], relative: bool, scale: Scale) -> (String, Option<usize>) {
    let mut out = String::new();
    let mut x: Fixed = 0;
    let mut y: Fixed = 0;
    let mut index = 0;

    while index < parts.len() {
        let command = parts[index].chars().nth(0).unwrap();
        index += 1;
        if !is_command(command) {
            return (out, Some(index - 1));
        }

        let mut letter = command;
        loop {
            let points = match letter {
                'a' | 'A' => 1,
                'm' | 'l' | 't' | 'h' | 'v' | 'M' | 'L' | 'T' | 'H' | 'V' => 1,
                's' | 'q' | 'S' | 'Q' => 2,
                'c' | 'C' => 3,
                'z' | 'Z' => 0,
                _ => panic!("command {letter}, index {}", index - 1),
            };
            let arc = matches!(letter, 'a' | 'A');
            let pair = if matches!(letter, 'h' | 'v' | 'H' | 'V') { 1 } else { 2 };
            let needed = if arc { 5 } else { 0 } + points * pair;

            // an incomplete or unreadable parameter set ends the path, the way SVG parsers read it
            if index + needed > parts.len() {
                return (out, Some(parts.len()));
            }
            let mut numbers = Vec::with_capacity(needed);
            for offset in 0..needed {
                match fixed::parse(&parts[index + offset]) {
                    Ok(number) => numbers.push(number),
                    Err(_) => return (out, Some(index + offset)),
                }
            }

            if relative {
                out.push(letter.to_ascii_lowercase());
            } else {
                out.push(letter.to_ascii_uppercase());
            }

            if arc {
                let rx = scale.x.apply(numbers[0]);
                write_coordinate(&mut out, rx, false);
                let ry = scale.y.apply(numbers[1]);
                write_coordinate(&mut out, ry, true);
                write_part(&mut out, &parts[index + 2], true);
                write_part(&mut out, &parts[index + 3], true);
                write_part(&mut out, &parts[index + 4], true);
            }

            let mut to_x = x;
            let mut to_y = y;

            for point in 0..points {
                let base = (if arc { 5 } else { 0 }) + point * pair;
                match letter {
                    'h' => { to_x = x + scale.x.apply(numbers[base]); }
                    'v' => { to_y = y + scale.y.apply(numbers[base]); }
                    'H' => { to_x = scale.x.apply(numbers[base]); }
                    'V' => { to_y = scale.y.apply(numbers[base]); }
                    'm' | 'l' | 't' | 'c' | 's' | 'q' | 'a' => {
                        to_x = x + scale.x.apply(numbers[base]);
                        to_y = y + scale.y.apply(numbers[base + 1]);
                    }
                    'M' | 'L' | 'T' | 'C' | 'S' | 'Q' | 'A' => {
                        to_x = scale.x.apply(numbers[base]);
                        to_y = scale.y.apply(numbers[base + 1]);
                    }
                    _ => panic!(),
                }

                let hv = matches!(letter, 'h' | 'v' | 'H' | 'V');
                let a = point > 0 || arc;
                if "mahlcsqtMAHLCSQT".contains(letter) {
                    write_coordinate(&mut out, if relative { to_x - x } else { to_x }, a);
                }
                if "mavlcsqtMAVLCSQT".contains(letter) {
                    write_coordinate(&mut out, if relative { to_y - y } else { to_y }, !hv);
                }
            }
            index += needed;
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
    (out, None)
}

/// Renders one path, warning about the dropped tail when the data is not a whole valid path.
fn path_data(parts: &[String], relative: bool, scale: Scale, source: &str) -> String {
    let (path, dropped) = build_path(parts, relative, scale);
    if let Some(index) = dropped {
        match parts.get(index) {
            Some(token) => eprintln!("{source}: \"{token}\" is not a path command, the tail was dropped"),
            None => eprintln!("{source}: the path ends in the middle of a command, the tail was dropped"),
        }
    }
    path
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
