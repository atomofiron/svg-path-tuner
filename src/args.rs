use clap::error::ErrorKind;
use clap::{CommandFactory, Parser};

use crate::coordination::Coordination;
use crate::scale::Scale;

/// Scales a vector path: reads one from stdin, or rewrites Android vector .xml files in place.
#[derive(Parser)]
#[command(version, about)]
pub struct Args {
    /// .xml files to scale and rewrite in place
    #[arg(value_name = "FILE", value_parser = xml_file)]
    pub files: Vec<String>,

    /// scale ratio, 2 or /2
    #[arg(short, long, default_value = "1")]
    pub scale: Scale,

    /// target coordination: r (relative) or a (absolute), required for files
    #[arg(short, long, value_enum)]
    pub target: Option<Coordination>,
}

impl Args {
    /// Files need an explicit target coordination, clap cannot express that as an argument rule.
    pub fn verify(&self) {
        if !self.files.is_empty() && self.target.is_none() {
            Self::command()
                .error(ErrorKind::MissingRequiredArgument, "-t r or -t a is required for files")
                .exit();
        }
    }
}

fn xml_file(value: &str) -> Result<String, String> {
    if value.to_ascii_lowercase().ends_with(".xml") {
        Ok(value.to_owned())
    } else {
        Err(format!("{value} is not an .xml file"))
    }
}
