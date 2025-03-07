#![feature(is_some_and)]

use std::fs::{self, File};
use std::io::{BufRead, BufReader, Result, Write};

macro_rules! template {
    () => {
        "\
// Generated by build.rs, DO NOT edit

/// Linux error codes defination.
#[repr(i32)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LinuxError {{
{0}\
}}

impl LinuxError {{
    pub fn as_str(&self) -> &'static str {{
        use self::LinuxError::*;
        match self {{
{1}        }}
    }}

    pub fn code(self) -> i32 {{
        -(self as i32)
    }}
}}
"
    };
}

fn main() {
    gen_linux_errno().unwrap();
}

fn gen_linux_errno() -> Result<()> {
    let mut enum_define = Vec::new();
    let mut detail_info = Vec::new();

    let file = File::open("src/errno.h")?;
    for line in BufReader::new(file).lines().filter_map(|l| l.ok()) {
        if line.starts_with("#define") {
            let mut iter = line.split_whitespace();
            if let Some(name) = iter.nth(1) {
                if let Some(num) = iter.next() {
                    let description = if let Some(pos) = line.find("/* ") {
                        String::from(line[pos + 3..].trim_end_matches(" */"))
                    } else {
                        format!("Error number {num}")
                    };
                    writeln!(enum_define, "    /// {description}\n    {name} = {num},")?;
                    writeln!(detail_info, "            {name} => \"{description}\",")?;
                }
            }
        }
    }

    fs::write(
        "src/linux_errno.rs",
        format!(
            template!(),
            String::from_utf8_lossy(&enum_define),
            String::from_utf8_lossy(&detail_info)
        ),
    )?;

    Ok(())
}
