use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::process::{Command, Stdio};
use texlab_protocol::*;
use texlab_workspace::Document;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, PartialEq, Eq, Clone, Default)]
pub struct EnglishDiagnosticsProvider {
    diagnostics_by_uri: HashMap<Uri, Vec<Diagnostic>>,
    last_lint_time: u64,
}

impl EnglishDiagnosticsProvider {
    pub fn get(&self, document: &Document) -> Vec<Diagnostic> {
        match self.diagnostics_by_uri.get(&document.uri) {
            Some(diagnostics) => diagnostics.to_owned(),
            None => Vec::new(),
        }
    }

    pub fn update(&mut self, uri: &Uri, text: &str) {
        if uri.scheme() != "file" {
            return;
        }
        let current_time = SystemTime::now();
        let since_the_epoch = current_time.duration_since(UNIX_EPOCH).expect("Time went backwards");
        let current_timestamp = since_the_epoch.as_secs();
        /* Every 10 seconds */
        if current_timestamp > self.last_lint_time + 10 {
            self.last_lint_time = current_timestamp;
            self.diagnostics_by_uri
            .insert(uri.clone(), lint(text).unwrap_or_default());
        }
    }
}

pub static LINE_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new("[&|#] ([a-zA-Z]+) ([0-9]+) ([0-9]+): ([a-zA-Z]+)").unwrap());

fn lint(text: &str) -> Option<Vec<Diagnostic>> {
    println!("Start running spell checker");
    let mut process = Command::new("hunspell")
        .args(&["-a", "-t", "-d", "en_US"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;

    let feed = text.to_owned() + "/n/n/0";
    process
        .stdin
        .take()
        .unwrap()
        .write_all(feed.as_bytes())
        .ok()?;

    let mut stdout = String::new();
    process
        .stdout
        .take()
        .unwrap()
        .read_to_string(&mut stdout)
        .ok()?;

    let mut diagnostics = Vec::new();
    for line in stdout.lines() {
        if line.is_empty() {
            continue;
        }
        let first = line.chars().next().unwrap();
        match first {
            '*' => {},
            '&' | '#' => {
                if let Some(captures) = LINE_REGEX.captures(line) {
                    let wrong_word = captures[1].to_owned();
                    let line = captures[2].parse::<u64>().unwrap() - 1;
                    let character = captures[3].parse::<u64>().unwrap();
                    let digit = wrong_word.len() as u64;
                    let message = "Maybe a spelling error, suggestion: ".to_owned() + &captures[4];
                    let range = Range::new_simple(line, character, line, character + digit);
                    diagnostics.push(Diagnostic {
                        source: Some("Spell Checker".into()),
                        code: None,
                        message,
                        severity: Some(DiagnosticSeverity::Information),
                        range,
                        related_information: None,
                    })
                }
            },
            _ => {
                /* silently ignored */
                continue;
            }
        }
        
    }
    println!("Spell Checker Ok.");
    Some(diagnostics)
}
