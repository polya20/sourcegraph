use lsp_server::{ExtractError, Request, RequestId};
use serde::{Deserialize, Serialize};

// Requests

// bfg/initialize

#[derive(Debug, Eq, PartialEq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeParams {
    pub client_name: String,
}

#[derive(Debug, Eq, PartialEq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeResponse {
    pub server_version: String,
}

pub enum Initialize {}

impl lsp_types::request::Request for Initialize {
    type Params = InitializeParams;
    type Result = InitializeResponse;
    const METHOD: &'static str = "bfg/initialize";
}

// bfg/contextAtPosition

#[derive(Debug, Eq, PartialEq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Position {
    pub line: usize,
    pub character: usize,
}

#[derive(Debug, Eq, PartialEq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextAtPositionParams {
    pub uri: String,
    pub content: String,
    pub position: Position,
    pub max_chars: usize,
    pub context_range: Option<Range>,
}

#[derive(Debug, Eq, PartialEq, Clone, Deserialize, Serialize, Hash)]
#[serde(rename_all = "camelCase")]
pub struct SymbolContextSnippet {
    pub file_name: String,
    pub symbol: String,
    pub content: String,
}

#[derive(Debug, Eq, PartialEq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileContextSnippet {
    pub file_name: String,
    pub content: String,
}

#[derive(Debug, Eq, PartialEq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextAtPositionResponse {
    pub symbols: Vec<SymbolContextSnippet>,
    pub files: Vec<FileContextSnippet>,
}

pub enum ContextAtPosition {}

impl lsp_types::request::Request for ContextAtPosition {
    type Params = ContextAtPositionParams;
    type Result = ContextAtPositionResponse;
    const METHOD: &'static str = "bfg/contextAtPosition";
}

// bfg/shutdown

#[derive(Debug, Eq, PartialEq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShutdownParams {
    pub client_name: String,
}

pub enum Shutdown {}

impl lsp_types::request::Request for Shutdown {
    type Params = ShutdownParams;
    type Result = ();
    const METHOD: &'static str = "bfg/shutdown";
}

// bfg/gitRevision/didChange

#[derive(Debug, Eq, PartialEq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitRevisionDidChangeParams {
    pub git_directory_uri: String,
}

pub enum GitRevisionDidChange {}

impl lsp_types::request::Request for GitRevisionDidChange {
    type Params = GitRevisionDidChangeParams;
    type Result = ();
    const METHOD: &'static str = "bfg/gitRevision/didChange";
}

// Notifications

// Casting

pub fn cast_request<R>(req: Request) -> Result<(RequestId, R::Params), ExtractError<Request>>
where
    R: lsp_types::request::Request,
    R::Params: serde::de::DeserializeOwned,
{
    req.extract(R::METHOD)
}

// pub fn cast_notification<N>(not: Notification) -> Result<N::Params, ExtractError<Notification>>
// where
//     N: lsp_types::notification::Notification,
//     N::Params: serde::de::DeserializeOwned,
// {
//     not.extract(N::METHOD)
// }
