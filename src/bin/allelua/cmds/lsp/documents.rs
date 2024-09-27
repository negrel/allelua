use std::collections::HashMap;

use anyhow::Context;
use tower_lsp::lsp_types::{
    TextDocumentContentChangeEvent, TextDocumentItem, Url, VersionedTextDocumentIdentifier,
};

use super::doc::Doc;

#[derive(Debug, Default)]
pub struct Documents {
    open_docs: HashMap<Url, Doc>,
}

impl Documents {
    pub fn open(&mut self, doc: TextDocumentItem) {
        self.open_docs.insert(doc.uri.clone(), doc.into());
    }

    pub fn change(
        &mut self,
        id: VersionedTextDocumentIdentifier,
        changes: Vec<TextDocumentContentChangeEvent>,
    ) -> anyhow::Result<()> {
        let doc = self
            .get_mut(id.uri)
            .context("can't change a closed document")?;

        doc.change(id.version, changes)
    }

    pub fn get(&self, url: Url) -> Option<&Doc> {
        self.open_docs.get(&url)
    }

    pub fn get_mut(&mut self, url: Url) -> Option<&mut Doc> {
        self.open_docs.get_mut(&url)
    }

    pub fn close(&mut self, url: Url) {
        self.open_docs.remove(&url);
    }
}
