use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::process::{Command, Stdio};
use texlab_protocol::*;
use texlab_workspace::Document;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, PartialEq, Eq, Clone, Default)]
pub struct LatexDiagnosticsProvider {
    diagnostics_by_uri: HashMap<Uri, Vec<Diagnostic>>,
    last_lint_time: u64,
}

impl LatexDiagnosticsProvider {
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
        /* Every one minute */
        if current_timestamp > self.last_lint_time + 60 {
            self.last_lint_time = current_timestamp;
            self.diagnostics_by_uri
            .insert(uri.clone(), lint(text).unwrap_or_default());
        }
    }
}

pub static LINE_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new("(\\d+):(\\d+):(\\d+):(\\w+):(\\w+):(.*)").unwrap());

fn lint(text: &str) -> Option<Vec<Diagnostic>> {
    let mut process = Command::new("chktex")
        .args(&["-I0", "-f%l:%c:%d:%k:%n:%m\n"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;

    process
        .stdin
        .take()
        .unwrap()
        .write_all(text.as_bytes())
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
        if let Some(captures) = LINE_REGEX.captures(line) {
            let line = captures[1].parse::<u64>().unwrap() - 1;
            let character = captures[2].parse::<u64>().unwrap() - 1;
            let digit = captures[3].parse::<u64>().unwrap();
            let kind = &captures[4];
            let code = &captures[5];
            let message = captures[6].to_owned();
            let range = Range::new_simple(line, character, line, character + digit);
            let severity = match kind {
                "Message" => DiagnosticSeverity::Information,
                "Warning" => DiagnosticSeverity::Warning,
                _ => DiagnosticSeverity::Error,
            };

            diagnostics.push(Diagnostic {
                source: Some("chktex".into()),
                code: Some(NumberOrString::String(code.to_owned())),
                message,
                severity: Some(severity),
                range,
                related_information: None,
            })
        }
    }
    Some(diagnostics)
}
