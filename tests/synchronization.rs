#![feature(async_await)]

use jsonrpc::server::ActionHandler;
use lsp_types::*;
use std::sync::Arc;
use texlab::scenario::Scenario;

async fn run_completion(
    scenario: &Scenario,
    file: &'static str,
    position: Position,
) -> Vec<Arc<CompletionItem>> {
    let params = CompletionParams {
        text_document: TextDocumentIdentifier::new(scenario.uri(file)),
        position,
        context: None,
    };
    scenario.server.stop_scanning().await;
    scenario.server.completion(params).await.unwrap().items
}

#[runtime::test(runtime_tokio::Tokio)]
async fn test_did_change() {
    let scenario = Scenario::new("synchronization/did_change").await;
    scenario.open("foo.tex").await;
    assert_eq!(
        run_completion(&scenario, "foo.tex", Position::new(0, 1))
            .await
            .len()
            > 0,
        false
    );

    let params = DidChangeTextDocumentParams {
        text_document: VersionedTextDocumentIdentifier::new(scenario.uri("foo.tex"), 0),
        content_changes: vec![TextDocumentContentChangeEvent {
            range: None,
            range_length: None,
            text: "\\".to_owned(),
        }],
    };
    scenario.server.did_change(params);
    scenario.server.execute_actions().await;
    assert!(!run_completion(&scenario, "foo.tex", Position::new(0, 1))
        .await
        .is_empty());
}

#[runtime::test(runtime_tokio::Tokio)]
async fn test_indexing() {
    let scenario = Scenario::new("synchronization/did_change").await;
    scenario.open("foo.tex").await;

    let mut path = scenario.directory.path().to_owned();
    path.push("bar.tex");
    std::fs::write(&path, "\\foo").unwrap();

    let params = DidChangeTextDocumentParams {
        text_document: VersionedTextDocumentIdentifier::new(scenario.uri("foo.tex"), 0),
        content_changes: vec![TextDocumentContentChangeEvent {
            range: None,
            range_length: None,
            text: "\\fo\n\\include{bar}".to_owned(),
        }],
    };
    scenario.server.did_change(params);
    scenario.server.execute_actions().await;
    let items = run_completion(&scenario, "foo.tex", Position::new(0, 1)).await;
    assert!(items.iter().any(|item| item.label == "foo"));
}