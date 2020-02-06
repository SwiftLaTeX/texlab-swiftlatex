mod bibtex;
// mod build;
mod latex;

pub use self::bibtex::BibtexErrorCode;

use self::bibtex::BibtexDiagnosticsProvider;
// use self::build::BuildDiagnosticsProvider;
use self::english::EnglishDiagnosticsProvider;
use self::latex::LatexDiagnosticsProvider;
use texlab_protocol::Diagnostic;
use texlab_workspace::Document;


#[derive(Debug, PartialEq, Eq, Clone, Default)]
pub struct DiagnosticsManager {
    // pub build: BuildDiagnosticsProvider,
    pub latex: LatexDiagnosticsProvider,
    pub bibtex: BibtexDiagnosticsProvider,
}

impl DiagnosticsManager {
    pub fn get(&self, document: &Document) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        // diagnostics.append(&mut self.build.get(document));
        diagnostics.append(&mut self.latex.get(document));
        diagnostics.append(&mut self.bibtex.get(document));
        diagnostics
    }
}
