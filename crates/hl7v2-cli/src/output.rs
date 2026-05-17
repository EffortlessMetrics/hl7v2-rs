//! Shared CLI output and diagnostic sinks.

use std::fmt;
use std::fs;
use std::path::PathBuf;

pub(crate) struct OutputOptions<'a> {
    pub(crate) output: Option<&'a PathBuf>,
    pub(crate) quiet: bool,
    no_color: bool,
}

impl<'a> OutputOptions<'a> {
    pub(crate) const fn new(output: Option<&'a PathBuf>, quiet: bool, no_color: bool) -> Self {
        Self {
            output,
            quiet,
            no_color,
        }
    }

    pub(crate) fn emit(&self, output: &str) -> Result<(), Box<dyn std::error::Error>> {
        let _colors_enabled = !self.no_color;
        if let Some(path) = self.output {
            fs::write(path, output)?;
        } else {
            println!("{}", output);
        }
        Ok(())
    }

    pub(crate) fn emit_raw(&self, output: &str) -> Result<(), Box<dyn std::error::Error>> {
        let _colors_enabled = !self.no_color;
        if let Some(path) = self.output {
            fs::write(path, output)?;
        } else {
            print!("{output}");
        }
        Ok(())
    }

    pub(crate) fn diagnostic(&self, message: impl fmt::Display) {
        let _colors_enabled = !self.no_color;
        if !self.quiet {
            eprintln!("{message}");
        }
    }
}
