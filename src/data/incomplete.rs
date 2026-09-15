/// A command that came without its parameters, the parser leaves it out and keeps the rest.
pub struct Incomplete {
    pub letter: char,
    pub got: usize,
    pub needed: usize,
}