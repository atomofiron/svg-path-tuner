use clap::ValueEnum;

#[derive(Clone, Copy, PartialEq, ValueEnum)]
pub enum Coordination {
    #[value(name = "r", alias = "relative")]
    Relative,
    #[value(name = "a", alias = "absolute")]
    Absolute,
}
