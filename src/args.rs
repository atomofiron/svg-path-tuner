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
    files: Vec<String>,

    /// target viewport size, W or WxH, the ratios come from each file viewport
    #[arg(short, long)]
    size: Option<Size>,

    /// target coordination: r (relative) or a (absolute), required for files
    #[arg(short, long, value_enum)]
    target: Option<Coordination>,
}

/// What the arguments ask for.
pub enum Mode {
    /// One path per line from stdin, both coordinations are printed without a target.
    Stdin { coordination: Option<Coordination> },
    /// Files scaled and rewritten in place.
    Files { coordination: Coordination, files: Vec<String>, size: Option<Size> },
}

impl Args {
    /// Rules clap cannot express: files need a target, a size needs files.
    pub fn into_mode(self) -> Result<Mode, clap::Error> {
        if self.files.is_empty() {
            if self.size.is_some() {
                return Err(Self::command().error(
                    ErrorKind::InvalidValue,
                    "-s is a viewport size, it needs .xml files to compute the ratios from",
                ));
            }
            return Ok(Mode::Stdin { coordination: self.target });
        }
        let Some(coordination) = self.target else {
            return Err(Self::command().error(
                ErrorKind::MissingRequiredArgument,
                "-t r or -t a is required for files",
            ));
        };
        Ok(Mode::Files { coordination, files: self.files, size: self.size })
    }
}

fn xml_file(value: &str) -> Result<String, String> {
    if value.to_ascii_lowercase().ends_with(".xml") {
        Ok(value.to_owned())
    } else {
        Err(format!("{value} is not an .xml file"))
    }
}
