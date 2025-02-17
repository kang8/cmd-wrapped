use std::{
    env,
    error::Error,
    fs::File,
    io::{BufRead, BufReader, Cursor, Read},
    process::Command,
};

use crate::view::View;

#[derive(Debug, Clone)]
pub enum HistoryProvider {
    Zsh,
    Bash,
    Atuin,
    Fish,
}

impl HistoryProvider {
    pub fn from(shell: &String) -> Self {
        match shell.as_str() {
            "zsh" => Self::Zsh,
            "bash" => {
                View::clear();
                View::content("It appears that you are using Bash");
                View::content(
                    "If you haven't configured the $HISTTIMEFORMAT for Bash, the time-related statistics may be INVALID :(",
                );
                View::content("(but other components will remain unaffected.)");
                View::content("Press [Enter] to continue");
                View::wait();
                Self::Bash
            }
            "atuin" => Self::Atuin,
            "fish" => Self::Fish,
            _ => {
                View::content(&format!(
                    "Sorry, {} is not supported yet\n\n",
                    shell.split('/').last().unwrap_or("")
                ));
                std::process::exit(1);
            }
        }
    }

    pub fn history_stream(&self) -> Result<Box<dyn Read>, Box<dyn Error>> {
        match self {
            HistoryProvider::Zsh | HistoryProvider::Bash => {
                let history_file_name = match self {
                    HistoryProvider::Zsh => ".zsh_history",
                    HistoryProvider::Bash => ".bash_history",
                    _ => unreachable!(),
                };
                let file_path = format!("{}/{}", env::var("HOME")?, history_file_name);
                Ok(Box::new(File::open(file_path)?))
            }
            HistoryProvider::Atuin => {
                let output = Command::new("atuin")
                    .args(["history", "list", "--format", "{time};{command}"])
                    .output()?;
                Ok(Box::new(Cursor::new(output.stdout)))
            }
            HistoryProvider::Fish => {
                let output = Command::new("fish")
                    .arg("-c")
                    .arg("history -show-time='%s;'")
                    .output()?;
                Ok(Box::new(Cursor::new(output.stdout)))
            }
        }
    }
}

pub struct History {
    buff_reader: BufReader<Box<dyn Read>>,
    shell_type: HistoryProvider,
}

impl History {
    pub fn from(shell: &HistoryProvider) -> Result<Self, Box<dyn Error>> {
        Ok(History {
            shell_type: shell.clone(),
            buff_reader: BufReader::new(shell.history_stream()?),
        })
    }
}

impl Iterator for History {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        match self.shell_type {
            HistoryProvider::Zsh | HistoryProvider::Atuin | HistoryProvider::Fish => {
                let mut block = String::new();
                let mut buf = vec![];
                loop {
                    self.buff_reader.read_until(b'\n', &mut buf).unwrap();
                    if buf.is_empty() {
                        return if block.is_empty() { None } else { Some(block) };
                    }
                    let str = String::from_utf8_lossy(&buf).trim().to_owned();
                    block += &str;
                    if str.is_empty() {
                        buf.clear();
                        continue;
                    }
                    if str.ends_with('\\') {
                        block = block.strip_suffix('\\')?.into();
                        buf.clear();
                        continue;
                    }
                    break Some(block);
                }
            }
            HistoryProvider::Bash => {
                let mut block = String::new();
                let mut buf = vec![];
                loop {
                    self.buff_reader.read_until(b'\n', &mut buf).unwrap();
                    if buf.is_empty() {
                        return None;
                    }
                    let str = String::from_utf8_lossy(&buf).to_owned();
                    block += &str;
                    if str.starts_with('#') {
                        buf.clear();
                        continue;
                    }
                    break Some(block.trim().into());
                }
            }
        }
    }
}
