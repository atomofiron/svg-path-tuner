use std::fmt::Display;
use std::io;
use std::io::Write;

pub trait PrintExt {
    fn print(&self);
    fn println(&self);
    fn eprintln(&self);
}

impl<E: Display> PrintExt for E {

    fn print(&self) {
        print!("{self}");
        io::stdout().flush().unwrap();
    }

    fn println(&self) {
        println!("{self}");
    }

    fn eprintln(&self) {
        eprintln!("{self}");
    }
}
