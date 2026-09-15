pub mod path;
pub mod print;

use std::error::Error;

pub type Rslt<T> = Result<T, Box<dyn Error>>;

