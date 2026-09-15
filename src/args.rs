use clap::error::ErrorKind;
use clap::{CommandFactory, Parser};

use crate::coordination::Coordination;
use crate::size::Size;

/// Scales a vector path: reads one from stdin, or rewrites Android vector .xml files in place.
#[derive(Parser)]
#[command(version, about)]
pub struct Args {
    /// .xml files to scale and rewrite in place
    #[arg(value_name = "FILE", value_parser = xml_file)]
    pub files: Vec<String>,

    /// target viewport size, W or WxH, the ratios come from each file viewport
    #[arg(short, long)]
    pub size: Option<Size>,

    /// target coordination: r (relative) or a (absolute), required for files
    #[arg(short, long, value_enum)]
    pub target: Option<Coordination>,
}

impl Args {
    /// Rules clap cannot express: files need a target, a size needs files.
    pub fn verify(&self) {
        if !self.files.is_empty() && self.target.is_none() {
            Self::command()
                .error(ErrorKind::MissingRequiredArgument, "-t r or -t a is required for files")
                .exit();
        }
        if self.files.is_empty() && self.size.is_some() {
            Self::command()
                .error(
                    ErrorKind::InvalidValue,
                    "-s is a viewport size, it needs .xml files to compute the ratios from",
                )
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
