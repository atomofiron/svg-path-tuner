use crate::args::coordination::Coordination;
use crate::args::size::Size;

/// What the arguments ask for.
pub enum Mode {
    /// One path per line from stdin, both coordinations are printed without a target.
    Stdin { coordination: Option<Coordination> },
    /// Inputs scaled and rewritten in place, a folder gives the .xml files in it.
    Files { coordination: Coordination, files: Vec<String>, size: Option<Size> },
}
