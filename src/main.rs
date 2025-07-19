use std::io::{self, Write};
use std::process::exit;

const SEPARATOR: char = ' ';

fn main() {
    loop {
        work();
    }
}

fn work() {
    print!("input path: ");
    io::stdout().flush().unwrap();

    let mut line = String::new();
    io::stdin().read_line(&mut line).unwrap();
    if line.len() == 1 && line.chars().nth(0).unwrap() == '\n' {
        exit(0);
    }
    let line = line.trim();

    let mut parts = Vec::new();
    let mut part = String::new();
    let mut continue_part = false;

    for c in line.chars() {
        if continue_part {
            continue_part = false;
            if c == '-' || c == '+' { // 1.05E-7 1.05E+7
                part.push(c);
                continue;
            }
        }
        match c {
            'm' | 'a' | 'h' | 'v' | 'l' | 'c' | 's' | 'q' | 't' | 'z' |
            'M' | 'A' | 'H' | 'V' | 'L' | 'C' | 'S' | 'Q' | 'T' | 'Z' |
            ',' | ' ' | '-' => {
                if !part.is_empty() {
                    parts.push(part.clone());
                    part.clear();
                }
                match c {
                    ' ' | ',' => {}
                    '-' => part.push('-'),
                    _ => parts.push(c.to_string()),
                }
            }
            '0'..='9' | '.' | 'e' | 'E' => {
                part.push(c);
                if c == 'e' || c == 'E' {
                    continue_part = true;
                }
            }
            _ => {}
        }
    }
    if !part.is_empty() {
        parts.push(part);
    }
    println!("parts: {}", parts.join(", "));
    print_parts(&parts, true);
    print_parts(&parts, false);
}

fn print_parts(parts: &[String], relative: bool) {
    let mut x = 0.0;
    let mut y = 0.0;
    let mut index = 0;

    while index < parts.len() {
        let command = parts[index].chars().next().unwrap();
        index += 1;

        if relative {
            print!("{}", command.to_ascii_lowercase());
        } else {
            print!("{}", command.to_ascii_uppercase());
        }

        let mut to_x = x;
        let mut to_y = y;

        let points = match command {
            'a' | 'A' => {
                print_part(&parts[index], false);
                index += 1;
                print_part(&parts[index], true);
                index += 1;
                print_part(&parts[index], true);
                index += 1;
                print_part(&parts[index], true);
                index += 1;
                print_part(&parts[index], true);
                index += 1;
                1
            }
            'm' | 'l' | 't' | 'h' | 'v' | 'M' | 'L' | 'T' | 'H' | 'V' => 1,
            's' | 'q' | 'S' | 'Q' => 2,
            'c' | 'C' => 3,
            'z' | 'Z' => 0,
            _ => panic!("command {command}, index {}", index - 1),
        };

        for _ in 0..points {
            match command {
                'h' => { to_x = x + parts[index].parse::<f32>().unwrap(); index += 1; }
                'v' => { to_y = y + parts[index].parse::<f32>().unwrap(); index += 1; }
                'H' => { to_x = parts[index].parse::<f32>().unwrap(); index += 1; }
                'V' => { to_y = parts[index].parse::<f32>().unwrap(); index += 1; }
                'm' | 'l' | 't' | 'c' | 's' | 'q' | 'a' => {
                    to_x = x + parts[index].parse::<f32>().unwrap(); index += 1;
                    to_y = y + parts[index].parse::<f32>().unwrap(); index += 1;
                }
                'M' | 'L' | 'T' | 'C' | 'S' | 'Q' | 'A' => {
                    to_x = parts[index].parse::<f32>().unwrap(); index += 1;
                    to_y = parts[index].parse::<f32>().unwrap(); index += 1;
                }
                _ => panic!(),
            }

            let hv = matches!(command, 'h' | 'v' | 'H' | 'V');
            if relative {
                if "mhlcsqtMHLCSQT".contains(command) {
                    print_coordinate(to_x - x, false);
                }
                if "mvlcsqtMVLCSQT".contains(command) {
                    print_coordinate(to_y - y, !hv);
                }
            } else {
                if "mhlcsqtMHLCSQT".contains(command) {
                    print_coordinate(to_x, false);
                }
                if "mvlcsqtMVLCSQT".contains(command) {
                    print_coordinate(to_y, !hv);
                }
            }
        }
        x = fix(to_x);
        y = fix(to_y);
    }
    println!();
}

fn fix(v: f32) -> f32 {
    (v * 10_000_000.0).round() / 10_000_000.0
}

fn print_part(part: &str, allow_separator: bool) {
    if allow_separator && !part.starts_with('-') {
        print!("{}", SEPARATOR);
    }
    if part.starts_with('.') || part.starts_with("-.") || !part.ends_with(".0") {
        print!("{}", part);
    } else {
        print!("{}", &part[..part.len() - 2]);
    }
}

fn print_coordinate(value: f32, allow_separator: bool) {
    let calced = fix(value);
    if allow_separator && calced >= 0.0 {
        print!("{}", SEPARATOR);
    }
    match calced.fract() {
        0.0 => print!("{}", calced as i32),
        _ => print!("{}", calced),
    }
}
