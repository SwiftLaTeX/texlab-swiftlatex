use crate::formatting::bibtex::{self, BibtexFormattingParams};
use crate::syntax::*;
use crate::workspace::*;
use lsp_types::*;
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::Path;

static WHITESPACE_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new("\\s+").unwrap());

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub enum CompletionItemData {
    Command,
    CommandSnippet,
    Environment,
    Label,
    Folder,
    File,
    PgfLibrary,
    TikzLibrary,
    Color,
    ColorModel,
    Package,
    Class,
    EntryType,
    FieldName,
    Citation { uri: Uri, key: String },
    Argument,
}

impl Into<serde_json::Value> for CompletionItemData {
    fn into(self) -> serde_json::Value {
        serde_json::to_value(self).unwrap()
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum LatexComponentId<'a> {
    User,
    Component(Vec<&'a str>),
}

impl<'a> LatexComponentId<'a> {
    pub fn kernel() -> Self {
        LatexComponentId::Component(vec![])
    }

    pub fn detail(&self) -> String {
        match self {
            LatexComponentId::User => "user-defined".to_owned(),
            LatexComponentId::Component(files) => {
                if files.is_empty() {
                    "built-in".to_owned()
                } else {
                    files.join(", ")
                }
            }
        }
    }
}

fn supports_images(request: &FeatureRequest<CompletionParams>) -> bool {
    request
        .client_capabilities
        .text_document
        .as_ref()
        .and_then(|cap| cap.completion.as_ref())
        .and_then(|cap| cap.completion_item.as_ref())
        .and_then(|cap| cap.documentation_format.as_ref())
        .map_or(true, |formats| formats.contains(&MarkupKind::Markdown))
}

pub fn command(
    request: &FeatureRequest<CompletionParams>,
    name: String,
    image: Option<&str>,
    glyph: Option<&str>,
    text_edit: TextEdit,
    component: &LatexComponentId,
) -> CompletionItem {
    let detail = glyph.map_or_else(
        || component.detail(),
        |glyph| format!("{}, {}", glyph, component.detail()),
    );
    CompletionItem {
        kind: Some(adjust_kind(request, CompletionItemKind::Function)),
        data: Some(CompletionItemData::Command.into()),
        documentation: image.and_then(|image| image_documentation(&request, &name, image)),
        text_edit: Some(text_edit),
        ..CompletionItem::new_simple(name, detail)
    }
}

pub fn command_snippet(
    request: &FeatureRequest<CompletionParams>,
    name: &'static str,
    image: Option<&str>,
    template: &'static str,
    component: &LatexComponentId,
) -> CompletionItem {
    CompletionItem {
        kind: Some(adjust_kind(request, CompletionItemKind::Snippet)),
        data: Some(CompletionItemData::CommandSnippet.into()),
        documentation: image.and_then(|image| image_documentation(&request, &name, image)),
        insert_text: Some(template.into()),
        insert_text_format: Some(InsertTextFormat::Snippet),
        ..CompletionItem::new_simple(name.into(), component.detail())
    }
}

pub fn environment(
    request: &FeatureRequest<CompletionParams>,
    name: String,
    text_edit: TextEdit,
    component: &LatexComponentId,
) -> CompletionItem {
    CompletionItem {
        kind: Some(adjust_kind(request, CompletionItemKind::EnumMember)),
        data: Some(CompletionItemData::Environment.into()),
        text_edit: Some(text_edit),
        ..CompletionItem::new_simple(name, component.detail())
    }
}

pub fn label(
    request: &FeatureRequest<CompletionParams>,
    name: String,
    text_edit: TextEdit,
    context: Option<&OutlineContext>,
) -> CompletionItem {
    let kind = match context.as_ref().map(|ctx| &ctx.item) {
        Some(OutlineContextItem::Section(_)) => CompletionItemKind::Module,
        Some(OutlineContextItem::Caption { .. }) => CompletionItemKind::Method,
        Some(OutlineContextItem::Theorem { .. }) => CompletionItemKind::Class,
        Some(OutlineContextItem::Equation) => CompletionItemKind::Constant,
        Some(OutlineContextItem::Item) => CompletionItemKind::EnumMember,
        None => CompletionItemKind::Field,
    };

    let detail = context.as_ref().and_then(|ctx| ctx.detail());

    let filter_text = context
        .as_ref()
        .map(|ctx| format!("{} {}", name, ctx.reference()));

    let documentation = context
        .and_then(|ctx| match &ctx.item {
            OutlineContextItem::Caption { text, .. } => Some(text.clone()),
            _ => None,
        })
        .map(Documentation::String);

    CompletionItem {
        label: name,
        kind: Some(adjust_kind(request, kind)),
        data: Some(CompletionItemData::Label.into()),
        text_edit: Some(text_edit),
        filter_text,
        detail,
        documentation,
        ..CompletionItem::default()
    }
}

pub fn folder(
    request: &FeatureRequest<CompletionParams>,
    path: &Path,
    text_edit: TextEdit,
) -> CompletionItem {
    CompletionItem {
        label: path.file_name().unwrap().to_string_lossy().into_owned(),
        kind: Some(adjust_kind(request, CompletionItemKind::Folder)),
        data: Some(CompletionItemData::Folder.into()),
        text_edit: Some(text_edit),
        ..CompletionItem::default()
    }
}

pub fn file(
    request: &FeatureRequest<CompletionParams>,
    path: &Path,
    text_edit: TextEdit,
) -> CompletionItem {
    CompletionItem {
        label: path.file_name().unwrap().to_string_lossy().into_owned(),
        kind: Some(adjust_kind(request, CompletionItemKind::File)),
        data: Some(CompletionItemData::File.into()),
        text_edit: Some(text_edit),
        ..CompletionItem::default()
    }
}

pub fn pgf_library(
    request: &FeatureRequest<CompletionParams>,
    name: &'static str,
    text_edit: TextEdit,
) -> CompletionItem {
    CompletionItem {
        label: name.into(),
        kind: Some(adjust_kind(request, CompletionItemKind::Module)),
        data: Some(CompletionItemData::PgfLibrary.into()),
        text_edit: Some(text_edit),
        ..CompletionItem::default()
    }
}

pub fn tikz_library(
    request: &FeatureRequest<CompletionParams>,
    name: &'static str,
    text_edit: TextEdit,
) -> CompletionItem {
    CompletionItem {
        label: name.into(),
        kind: Some(adjust_kind(request, CompletionItemKind::Module)),
        data: Some(CompletionItemData::TikzLibrary.into()),
        text_edit: Some(text_edit),
        ..CompletionItem::default()
    }
}

pub fn color(
    request: &FeatureRequest<CompletionParams>,
    name: &'static str,
    text_edit: TextEdit,
) -> CompletionItem {
    CompletionItem {
        label: name.into(),
        kind: Some(adjust_kind(request, CompletionItemKind::Color)),
        data: Some(CompletionItemData::Color.into()),
        text_edit: Some(text_edit),
        ..CompletionItem::default()
    }
}

pub fn color_model(
    request: &FeatureRequest<CompletionParams>,
    name: &'static str,
    text_edit: TextEdit,
) -> CompletionItem {
    CompletionItem {
        label: name.into(),
        kind: Some(adjust_kind(request, CompletionItemKind::Color)),
        data: Some(CompletionItemData::ColorModel.into()),
        text_edit: Some(text_edit),
        ..CompletionItem::default()
    }
}

pub fn package(
    request: &FeatureRequest<CompletionParams>,
    name: &'static str,
    text_edit: TextEdit,
) -> CompletionItem {
    CompletionItem {
        label: name.into(),
        kind: Some(adjust_kind(request, CompletionItemKind::Class)),
        data: Some(CompletionItemData::Package.into()),
        text_edit: Some(text_edit),
        ..CompletionItem::default()
    }
}

pub fn class(
    request: &FeatureRequest<CompletionParams>,
    name: &'static str,
    text_edit: TextEdit,
) -> CompletionItem {
    CompletionItem {
        label: name.into(),
        kind: Some(adjust_kind(request, CompletionItemKind::Class)),
        data: Some(CompletionItemData::Class.into()),
        text_edit: Some(text_edit),
        ..CompletionItem::default()
    }
}

pub fn citation(
    request: &FeatureRequest<CompletionParams>,
    uri: Uri,
    entry: &BibtexEntry,
    key: String,
    text_edit: TextEdit,
) -> CompletionItem {
    let params = BibtexFormattingParams::default();
    let entry_code = bibtex::format_entry(&entry, &params);
    let filter_text = format!(
        "{} {}",
        &key,
        WHITESPACE_REGEX
            .replace_all(
                &entry_code
                    .replace('{', "")
                    .replace('}', "")
                    .replace(',', " ")
                    .replace('=', " "),
                " ",
            )
            .trim()
    );

    CompletionItem {
        label: key.to_owned(),
        kind: Some(adjust_kind(request, CompletionItemKind::Field)),
        filter_text: Some(filter_text),
        data: Some(CompletionItemData::Citation { uri, key }.into()),
        text_edit: Some(text_edit),
        ..CompletionItem::default()
    }
}

pub fn entry_type(
    request: &FeatureRequest<CompletionParams>,
    ty: &BibtexEntryTypeDoc,
    text_edit: TextEdit,
) -> CompletionItem {
    let kind = match &ty.category {
        BibtexEntryTypeCategory::Misc => CompletionItemKind::Interface,
        BibtexEntryTypeCategory::String => CompletionItemKind::Text,
        BibtexEntryTypeCategory::Article => CompletionItemKind::Event,
        BibtexEntryTypeCategory::Book => CompletionItemKind::Struct,
        BibtexEntryTypeCategory::Collection => CompletionItemKind::TypeParameter,
        BibtexEntryTypeCategory::Part => CompletionItemKind::Operator,
        BibtexEntryTypeCategory::Thesis => CompletionItemKind::Property,
    };

    CompletionItem {
        label: (&ty.name).into(),
        kind: Some(adjust_kind(request, kind)),
        data: Some(CompletionItemData::EntryType.into()),
        text_edit: Some(text_edit),
        documentation: ty.documentation.as_ref().map(|doc| {
            Documentation::MarkupContent(MarkupContent {
                kind: MarkupKind::Markdown,
                value: doc.into(),
            })
        }),
        ..CompletionItem::default()
    }
}

pub fn field_name(
    request: &FeatureRequest<CompletionParams>,
    field: &'static BibtexFieldDoc,
    text_edit: TextEdit,
) -> CompletionItem {
    CompletionItem {
        label: (&field.name).into(),
        kind: Some(adjust_kind(request, CompletionItemKind::Field)),
        data: Some(CompletionItemData::FieldName.into()),
        text_edit: Some(text_edit),
        documentation: Some(Documentation::MarkupContent(MarkupContent {
            kind: MarkupKind::Markdown,
            value: (&field.documentation).into(),
        })),
        ..CompletionItem::default()
    }
}

pub fn argument(
    request: &FeatureRequest<CompletionParams>,
    name: &'static str,
    text_edit: TextEdit,
    image: Option<&str>,
) -> CompletionItem {
    CompletionItem {
        label: name.into(),
        kind: Some(adjust_kind(request, CompletionItemKind::Field)),
        data: Some(CompletionItemData::Argument.into()),
        text_edit: Some(text_edit),
        documentation: image.and_then(|image| image_documentation(&request, &name, image)),
        ..CompletionItem::default()
    }
}

fn image_documentation(
    request: &FeatureRequest<CompletionParams>,
    name: &str,
    image: &str,
) -> Option<Documentation> {
    if supports_images(request) {
        Some(Documentation::MarkupContent(MarkupContent {
            kind: MarkupKind::Markdown,
            value: format!(
                "![{}](data:image/png;base64,{}|width=48,height=48)",
                name, image
            ),
        }))
    } else {
        None
    }
}

fn adjust_kind(
    request: &FeatureRequest<CompletionParams>,
    kind: CompletionItemKind,
) -> CompletionItemKind {
    if let Some(value_set) = request
        .client_capabilities
        .text_document
        .as_ref()
        .and_then(|cap| cap.completion.as_ref())
        .and_then(|cap| cap.completion_item_kind.as_ref())
        .and_then(|cap| cap.value_set.as_ref())
    {
        if value_set.contains(&kind) {
            return kind;
        }
    }
    CompletionItemKind::Text
}
