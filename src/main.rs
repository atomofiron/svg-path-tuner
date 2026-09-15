mod args;
mod data;
mod ext;
mod fixed;

use crate::args::Args;
use crate::data::incomplete::Incomplete;
use crate::data::stopped::Stopped;
use crate::ext::Rslt;
use crate::ext::path::is_xml;
use crate::ext::print::PrintExt;
use crate::fixed::Fixed;
use args::coordination::Coordination;
use args::mode::Mode;
use args::scale::Scale;
use args::size::Size;
use clap::Parser;
use std::fs;
use std::io;
use std::ops::Range;

const SEPARATOR: char = ' ';
const PATH_DATA: &str = "android:pathData";
const VIEWPORT_WIDTH: &str = "android:viewportWidth";
const VIEWPORT_HEIGHT: &str = "android:viewportHeight";

fn main() -> Rslt<()> {
    let mode = match Args::try_parse().and_then(Args::into_mode) {
        Ok(mode) => mode,
        Err(error) => error.exit(), // clap prints the error itself, keeping clap own exit code
    };

    match mode {
        Mode::Stdin { coordination } => work(coordination),
        Mode::Files { coordination, files, size } => {
            let mut total = 0;
            let mut skipped = 0;
            for input in &files {
                let expanded = xml_files(input);
                if expanded.is_empty() {
                    eprintln!("{input}: no .xml files in it");
                    continue;
                }
                total += expanded.len();
                for file in &expanded {
                    if let Err(error) = tune_file(file, size, coordination) {
                        error.eprintln();
                        skipped += 1;
                    }
                }
            }
            match skipped {
                0 => Ok(()),
                skipped => Err(format!("{skipped} of {total} files were skipped").into()),
            }
        }
    }
}

/// Expands one input into .xml files: a folder gives its own .xml children, without recursion.
fn xml_files(input: &str) -> Vec<String> {
    if !fs::metadata(input).is_ok_and(|metadata| metadata.is_dir()) {
        return vec![input.to_owned()]; // not a folder, the file is read as it is
    }
    let Ok(entries) = fs::read_dir(input) else {
        return Vec::new(); // the folder is there but cannot be listed
    };
    let mut files: Vec<String> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_file() && is_xml(path))
        .map(|path| path.to_string_lossy().into_owned())
        .collect();
    files.sort();
    files
}

/// Renders the paths read from stdin one per line, an empty line or the end of the input ends it.
fn work(coordination: Option<Coordination>) -> Rslt<()> {
    let stdin = io::stdin();
    let mut line = String::new();
    "Ctrl-C to exit".println();
    loop {
        "input path: ".print();

        line.clear();
        stdin.read_line(&mut line)
            .map_err(|error| format!("stdin: {error}"))?;
        if line.is_empty() {
            return Ok(()); // the end of the input ends the loop
        }
        if line.trim().is_empty() {
            continue; // a blank line carries no path
        }

        let parts = tokenize(line.trim());
        "parts:".println();
        parts.join(", ").println();
        let scale = Scale::identity();
        match coordination {
            Some(Coordination::Relative) => path_data(&parts, true, scale, "input path")?.println(),
            Some(Coordination::Absolute) => path_data(&parts, false, scale, "input path")?.println(),
            None => {
                let relative = path_data(&parts, true, scale, "input path")?;
                let absolute = path_data(&parts, false, scale, "input path")?;
                "relative:".println();
                relative.println();
                "absolute:".println();
                absolute.println();
            }
        }
    }
}

/// Rewrites one file in place, an error when it was skipped and nothing was written.
fn tune_file(file: &str, size: Option<Size>, coordination: Coordination) -> Rslt<()> {
    let xml = fs::read_to_string(file).map_err(|error| format!("{file}: {error}, the file was skipped"))?;
    if !xml.contains("<vector") {
        eprintln!("{file}: no <vector> in the file, the file was skipped");
        return Ok(());
    }
    // the path data of such a file lives in strings.xml, the viewport alone cannot be scaled
    if xml.contains("android:pathData=\"@string") {
        eprintln!("{file}: it references @string, the file was skipped");
        return Ok(());
    }
    let scale = Scale::fit(read_viewport(&xml), size)
        .map_err(|error| format!("{file}: {error}, the file was skipped"))?;
    if !fits(scale, &xml) {
        return Err(format!(
            "{file}: the coordinates do not fit the fixed point math at {scale}, the file was skipped"
        )
        .into());
    }
    let (xml, paths) = map_attribute(&xml, PATH_DATA, |value| {
        scale_path_data(value, file, scale, coordination)
    })
    .map_err(|error| format!("{error}, the file was skipped"))?;
    let (xml, viewports) = match size {
        Some(size) => {
            let (xml, widths) = map_attribute(&xml, VIEWPORT_WIDTH, |_| Ok(fixed::format(size.width)))?;
            let (xml, heights) = map_attribute(&xml, VIEWPORT_HEIGHT, |_| Ok(fixed::format(size.height)))?;
            (xml, widths + heights)
        }
        None => (xml, 0),
    };
    fs::write(file, xml).map_err(|error| format!("{file}: {error}, the file was not written"))?;
    println!("{file}: {paths} pathData, {viewports} viewport, scale {scale}");
    Ok(())
}

/// Reads the viewport of the file, the ratios are computed against it.
fn read_viewport(xml: &str) -> Option<(Fixed, Fixed)> {
    let width = fixed::parse(attribute_value(xml, VIEWPORT_WIDTH)?).ok()?;
    let height = fixed::parse(attribute_value(xml, VIEWPORT_HEIGHT)?).ok()?;
    Some((width, height))
}

/// Whether the fixed point math holds for the coordinates of the file at the given scale.
fn fits(scale: Scale, xml: &str) -> bool {
    let total = attribute_values(xml, PATH_DATA)
        .into_iter()
        .flat_map(|range| tokenize(&xml[range]))
        .filter_map(|token| fixed::parse(&token).ok())
        .fold(0_i128, |sum, value| sum + i128::from(value).abs());
    // a relative coordinate is the difference of two positions, so the positions have to fit twice over
    let total = i64::try_from(total.saturating_mul(2)).unwrap_or(i64::MAX);
    scale.x.fits(total) && scale.y.fits(total)
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

        // an attribute that is commented out is not an attribute
        if let Some(comment) = xml[position..].find("<!--").map(|found| position + found)
            && comment < name
        {
            position = match xml[comment..].find("-->") {
                Some(end) => comment + end + 3,
                None => xml.len(),
            };
            continue;
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
fn map_attribute(xml: &str, attribute: &str, mut map: impl FnMut(&str) -> Rslt<String>) -> Rslt<(String, usize)> {
    let ranges = attribute_values(xml, attribute);
    if ranges.is_empty() {
        return Ok((xml.to_owned(), 0));
    }
    let mut out = String::with_capacity(xml.len());
    let mut position = 0;
    for range in &ranges {
        out.push_str(&xml[position..range.start]);
        out.push_str(&map(&xml[range.clone()])?);
        position = range.end;
    }
    out.push_str(&xml[position..]);
    Ok((out, ranges.len()))
}

fn scale_path_data(value: &str, file: &str, scale: Scale, coordination: Coordination) -> Rslt<String> {
    let parts = tokenize(value);
    if parts.is_empty() {
        return Ok(value.to_owned());
    }
    let path = path_data(&parts, coordination == Coordination::Relative, scale, file)?;
    if path.is_empty() {
        return Ok(value.to_owned()); // nothing scalable in there, the original text stays
    }
    Ok(path)
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

/// Whether the token is a command letter rather than a parameter.
fn is_command_token(token: &str) -> bool {
    token.chars().next().is_some_and(is_command)
}

fn build_path(parts: &[String], relative: bool, scale: Scale) -> (String, Option<Stopped>, Vec<Incomplete>) {
    let mut out = String::new();
    let mut incomplete = Vec::new();
    let mut x: Fixed = 0;
    let mut y: Fixed = 0;
    let mut index = 0;

    while index < parts.len() {
        let Some(command) = parts[index].chars().next() else {
            return (out, Some(Stopped::Command(index)), incomplete);
        };
        index += 1;
        if !is_command(command) {
            return (out, Some(Stopped::Command(index - 1)), incomplete);
        }

        let mut letter = command;
        loop {
            let points = match letter {
                'a' | 'A' => 1,
                'm' | 'l' | 't' | 'h' | 'v' | 'M' | 'L' | 'T' | 'H' | 'V' => 1,
                's' | 'q' | 'S' | 'Q' => 2,
                'c' | 'C' => 3,
                'z' | 'Z' => 0,
                _ => return (out, Some(Stopped::Command(index - 1)), incomplete),
            };
            let arc = matches!(letter, 'a' | 'A');
            let pair = if matches!(letter, 'h' | 'v' | 'H' | 'V') { 1 } else { 2 };
            let needed = if arc { 5 } else { 0 } + points * pair;

            // a command letter that takes the place of a parameter starts the next command, the way
            // Android reads it: this one is left out and the rest of the path is kept
            let available = (0..needed)
                .take_while(|offset| parts.get(index + offset).is_some_and(|token| !is_command_token(token)))
                .count();
            if available < needed {
                if index + available == parts.len() {
                    return (out, Some(Stopped::End), incomplete); // the data ends in the middle of a set
                }
                incomplete.push(Incomplete { letter, got: available, needed });
                index += available;
                break;
            }
            let mut numbers = Vec::with_capacity(needed);
            for offset in 0..needed {
                match fixed::parse(&parts[index + offset]) {
                    Ok(number) => numbers.push(number),
                    Err(_) => return (out, Some(Stopped::Parameter(index + offset)), incomplete),
                }
            }
            // scale the lengths up front, one axis each, arc rotation and flags stay as written
            for (offset, number) in numbers.iter_mut().enumerate() {
                let axis = match letter {
                    'h' | 'H' => Some(scale.x),
                    'v' | 'V' => Some(scale.y),
                    'a' | 'A' if offset < 2 => Some(if offset == 0 { scale.x } else { scale.y }),
                    'a' | 'A' if offset < 5 => None, // rotation, large-arc, sweep
                    _ => Some(if (if arc { offset - 5 } else { offset }) % 2 == 0 { scale.x } else { scale.y }),
                };
                if let Some(axis) = axis {
                    match axis.apply(*number) {
                        Some(scaled) => *number = scaled,
                        None => return (out, Some(Stopped::Overflow), incomplete),
                    }
                }
            }

            if relative {
                out.push(letter.to_ascii_lowercase());
            } else {
                out.push(letter.to_ascii_uppercase());
            }

            if arc {
                write_coordinate(&mut out, numbers[0], false);
                write_coordinate(&mut out, numbers[1], true);
                write_part(&mut out, &parts[index + 2], true);
                write_part(&mut out, &parts[index + 3], true);
                write_part(&mut out, &parts[index + 4], true);
            }

            let mut to_x = x;
            let mut to_y = y;

            for point in 0..points {
                let base = (if arc { 5 } else { 0 }) + point * pair;
                let next = match letter {
                    'h' => (x.checked_add(numbers[base]), Some(y)),
                    'v' => (Some(x), y.checked_add(numbers[base])),
                    'H' => (Some(numbers[base]), Some(y)),
                    'V' => (Some(x), Some(numbers[base])),
                    'm' | 'l' | 't' | 'c' | 's' | 'q' | 'a' => {
                        (x.checked_add(numbers[base]), y.checked_add(numbers[base + 1]))
                    }
                    'M' | 'L' | 'T' | 'C' | 'S' | 'Q' | 'A' => (Some(numbers[base]), Some(numbers[base + 1])),
                    _ => return (out, Some(Stopped::Command(index - 1)), incomplete),
                };
                let (Some(next_x), Some(next_y)) = next else {
                    return (out, Some(Stopped::Overflow), incomplete);
                };
                to_x = next_x;
                to_y = next_y;

                let hv = matches!(letter, 'h' | 'v' | 'H' | 'V');
                let a = point > 0 || arc;
                if "mahlcsqtMAHLCSQT".contains(letter) {
                    let Some(value) = (if relative { to_x.checked_sub(x) } else { Some(to_x) }) else {
                        return (out, Some(Stopped::Overflow), incomplete);
                    };
                    write_coordinate(&mut out, value, a);
                }
                if "mavlcsqtMAVLCSQT".contains(letter) {
                    let Some(value) = (if relative { to_y.checked_sub(y) } else { Some(to_y) }) else {
                        return (out, Some(Stopped::Overflow), incomplete);
                    };
                    write_coordinate(&mut out, value, !hv);
                }
            }
            index += needed;
            x = to_x;
            y = to_y;

            // a command reuses its letter for the implicit parameter sets, moveto turns into lineto
            if points == 0 || index == parts.len() || parts[index].chars().next().is_some_and(is_command) {
                break;
            }
            letter = match letter {
                'm' => 'l',
                'M' => 'L',
                _ => letter,
            };
        }
    }
    (out, None, incomplete)
}

/// Renders one path, warning about the commands and tails the data did not spell out.
fn path_data(parts: &[String], relative: bool, scale: Scale, source: &str) -> Rslt<String> {
    let (path, stopped, incomplete) = build_path(parts, relative, scale);
    for skipped in &incomplete {
        eprintln!("{source}: \"{}\" has {} of {} parameters, the command was skipped", skipped.letter, skipped.got, skipped.needed);
    }
    match stopped {
        Some(Stopped::Parameter(index)) => {
            let token = parts.get(index).map_or("", String::as_str);
            eprintln!("{source}: \"{token}\" is not a number, the tail was dropped");
        }
        Some(Stopped::Command(index)) => {
            let token = parts.get(index).map_or("", String::as_str);
            eprintln!("{source}: \"{token}\" is not a path command, the tail was dropped");
        }
        Some(Stopped::End) => {
            eprintln!("{source}: the path ends in the middle of a command, the tail was dropped");
        }
        Some(Stopped::Overflow) => {
            return Err(format!("{source}: the coordinates do not fit the fixed point math at {scale}").into());
        }
        None => {}
    }
    Ok(path)
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
