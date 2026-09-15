/// Why the parser stopped before the end of the data.
pub enum Stopped {
    /// A token that is not a number took the place of a parameter.
    Parameter(usize),
    /// A token that is not a command took the place of a command.
    Command(usize),
    /// The data ends in the middle of a command.
    End,
    /// A coordinate left the fixed point range.
    Overflow,
}
