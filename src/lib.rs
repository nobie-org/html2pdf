#![forbid(unsafe_code)]
#![warn(clippy::perf)]
#![warn(clippy::pedantic)]
#![warn(missing_docs)]
#![allow(clippy::module_name_repetitions)]
#![doc = include_str!("../README.md")]

use std::fmt::Debug;
use std::num::ParseFloatError;
use std::path::Path;
use std::thread::sleep;
use std::time::Duration;
use std::{fs, io};

use anyhow::Result;
pub use headless_chrome::types::PrintToPdfOptions;
use headless_chrome::Browser;
pub use headless_chrome::LaunchOptions;
use humantime::format_duration;
use log::{debug, info};
use thiserror::Error;

mod cli;

pub use cli::*;

/// The html2pdf Error
#[derive(Error, Debug)]
pub enum Error {
    /// Invalid paper size
    #[error(
        "Invalid paper size {0}, expected a value in A4, Letter, A3, Tabloid, A2, A1, A0, A5, A6"
    )]
    InvalidPaperSize(String),
    /// Invalid margin definition
    #[error("Invalid margin definition, expected 1, 2, or 4 value, got {0}")]
    InvalidMarginDefinition(String),
    /// Invalid margin value
    #[error("Invalid margin value: {0}")]
    InvalidMarginValue(ParseFloatError),
    /// Headless chrome issue
    #[error("Oops, an error occurs with headless chrome: {0}")]
    HeadlessChromeError(String),
    /// I/O issue
    #[error("Oops, an error occurs with IO")]
    IoError {
        /// The source error
        #[from]
        source: io::Error,
    },
}

impl From<ParseFloatError> for Error {
    fn from(source: ParseFloatError) -> Self {
        Error::InvalidMarginValue(source)
    }
}

impl From<anyhow::Error> for Error {
    fn from(source: anyhow::Error) -> Self {
        Error::HeadlessChromeError(source.to_string())
    }
}

/// Run HTML to PDF with `headless_chrome`
///
/// # Errors
///
/// Could fail if there is I/O or Chrome headless issue
pub fn run(opt: &Options) -> Result<(), Error> {
    let input = opt.input().to_string_lossy();
    let output = if let Some(path) = opt.output() {
        path.clone()
    } else {
        let mut path = opt.input().clone();
        path.set_extension("pdf");
        path
    };

    html_to_pdf_to_file(&input, output, opt.into(), opt.into(), opt.wait())?;

    Ok(())
}

/// Run HTML to PDF with `headless_chrome`
///
/// # Panics
/// Sorry, no error handling, just panic
///
/// # Errors
///
/// Could fail if there is I/O or Chrome headless issue
pub fn html_to_pdf_to_file<O>(
    input: &str,
    output: O,
    pdf_options: PrintToPdfOptions,
    launch_options: LaunchOptions,
    wait: Option<Duration>,
) -> Result<(), Error>
where
    O: AsRef<Path> + Debug,
{
    let local_pdf = html_to_pdf(input, pdf_options, launch_options, wait)?;

    fs::write(output.as_ref(), local_pdf)?;

    Ok(())
}

/// Run HTML to PDF with `headless_chrome` returning the PDF as bytes
///
/// # Panics
/// Sorry, no error handling, just panic
///
/// # Errors
///
/// Could fail if there is I/O or Chrome headless issue
pub fn html_to_pdf(
    input: &str,
    pdf_options: PrintToPdfOptions,
    launch_options: LaunchOptions,
    wait: Option<Duration>,
) -> Result<Vec<u8>, Error> {
    let input = if input.starts_with("http") {
        input.to_string()
    } else {
        format!("file://{input}")
    };

    Ok(print_to_pdf(&input, pdf_options, launch_options, wait)?)
}

fn print_to_pdf(
    file_path: &str,
    pdf_options: PrintToPdfOptions,
    launch_options: LaunchOptions,
    wait: Option<Duration>,
) -> Result<Vec<u8>> {
    let browser = Browser::new(launch_options)?;
    let tab = browser.new_tab()?;
    let tab = tab.navigate_to(file_path)?.wait_until_navigated()?;

    if let Some(wait) = wait {
        info!("Waiting {} before export to PDF", format_duration(wait));
        sleep(wait);
    }

    debug!("Using PDF options: {:?}", pdf_options);
    let bytes = tab.print_to_pdf(Some(pdf_options))?;

    Ok(bytes)
}
