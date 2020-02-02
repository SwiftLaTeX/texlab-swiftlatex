use super::combinators::{self, ArgumentContext, Parameter};
use crate::factory;
use futures_boxed::boxed;
use std::sync::Arc;
use texlab_protocol::*;
use texlab_syntax::*;
use texlab_workspace::*;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct LatexLabelCompletionProvider;

impl FeatureProvider for LatexLabelCompletionProvider {
    type Params = CompletionParams;
    type Output = Vec<CompletionItem>;

    #[boxed]
    async fn execute<'a>(&'a self, request: &'a FeatureRequest<Self::Params>) -> Self::Output {
        let parameters = LANGUAGE_DATA
            .label_commands
            .iter()
            .filter(|cmd| cmd.kind.is_reference())
            .map(|cmd| Parameter::new(&cmd.name, cmd.index));

        combinators::argument(request, parameters, |context| {
            async move {
                let options = &request.options;
                let source = Self::find_source(&context);
                let mut items = Vec::new();
                for document in request.related_documents() {
                    let workspace = Arc::clone(&request.view.workspace);
                    let view = DocumentView::new(workspace, Arc::clone(&document), options);
                    let outline = Outline::analyze(&view, options);

                    if let SyntaxTree::Latex(tree) = &document.tree {
                        for label in tree
                            .structure
                            .labels
                            .iter()
                            .filter(|label| label.kind == LatexLabelKind::Definition)
                            .filter(|label| Self::is_included(tree, label, source))
                        {
                            let outline_context = OutlineContext::parse(&view, &label, &outline);
                            for name in label.names() {
                                let text = name.text().to_owned();
                                let text_edit = TextEdit::new(context.range, text.clone());
                                let item = factory::label(
                                    request,
                                    text,
                                    text_edit,
                                    outline_context.as_ref(),
                                );
                                items.push(item);
                            }
                        }
                    }
                }
                items
            }
        })
        .await
    }
}

impl LatexLabelCompletionProvider {
    fn find_source(context: &ArgumentContext) -> LatexLabelReferenceSource {
        match LANGUAGE_DATA
            .label_commands
            .iter()
            .find(|cmd| cmd.name == context.parameter.name && cmd.index == context.parameter.index)
            .map(|cmd| cmd.kind)
            .unwrap()
        {
            LatexLabelKind::Definition => unreachable!(),
            LatexLabelKind::Reference(source) => source,
        }
    }

    fn is_included(
        tree: &LatexSyntaxTree,
        label: &LatexLabel,
        source: LatexLabelReferenceSource,
    ) -> bool {
        match source {
            LatexLabelReferenceSource::Everything => true,
            LatexLabelReferenceSource::Math => tree
                .env
                .environments
                .iter()
                .filter(|env| env.left.is_math())
                .any(|env| env.range().contains_exclusive(label.start())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inside_of_ref() {
        let items = test_feature(
            LatexLabelCompletionProvider,
            FeatureSpec {
                files: vec![
                    FeatureSpec::file(
                        "foo.tex",
                        "\\addbibresource{bar.bib}\\include{baz}\n\\ref{}",
                    ),
                    FeatureSpec::file("bar.bib", ""),
                    FeatureSpec::file("baz.tex", "\\label{foo}\\label{bar}\\ref{baz}"),
                ],
                main_file: "foo.tex",
                position: Position::new(1, 5),
                ..FeatureSpec::default()
            },
        );
        let labels: Vec<&str> = items.iter().map(|item| item.label.as_ref()).collect();
        assert_eq!(labels, vec!["foo", "bar"]);
    }

    #[test]
    fn outside_of_ref() {
        let items = test_feature(
            LatexLabelCompletionProvider,
            FeatureSpec {
                files: vec![
                    FeatureSpec::file("foo.tex", "\\include{bar}\\ref{}"),
                    FeatureSpec::file("bar.tex", "\\label{foo}\\label{bar}"),
                ],
                main_file: "foo.tex",
                position: Position::new(1, 6),
                ..FeatureSpec::default()
            },
        );
        assert!(items.is_empty());
    }

    #[test]
    fn eqref() {
        let items = test_feature(
            LatexLabelCompletionProvider,
            FeatureSpec {
                files: vec![FeatureSpec::file(
                    "foo.tex",
                    "\\begin{align}\\label{foo}\\end{align}\\label{bar}\n\\eqref{}",
                )],
                main_file: "foo.tex",
                position: Position::new(1, 7),
                ..FeatureSpec::default()
            },
        );
        let labels: Vec<&str> = items.iter().map(|item| item.label.as_ref()).collect();
        assert_eq!(labels, vec!["foo"]);
    }
}
