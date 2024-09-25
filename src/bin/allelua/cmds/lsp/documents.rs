use std::collections::HashMap;

use anyhow::{bail, Context};
use tower_lsp::lsp_types::{
    TextDocumentContentChangeEvent, TextDocumentItem, Url, VersionedTextDocumentIdentifier,
};

#[derive(Debug, Default)]
pub struct Documents {
    open_docs: HashMap<Url, TextDocumentItem>,
}

impl Documents {
    pub fn open(&mut self, doc: TextDocumentItem) {
        self.open_docs.insert(doc.uri.clone(), doc);
    }

    pub fn change(
        &mut self,
        id: VersionedTextDocumentIdentifier,
        changes: Vec<TextDocumentContentChangeEvent>,
    ) -> anyhow::Result<()> {
        let doc = self
            .get_mut(id.uri)
            .context("can't change a closed document")?;

        if changes.len() != 1 {
            bail!("only full TextDocumentSyncKind is supported")
        }

        if id.version <= doc.version {
            bail!("changes is older than stored document")
        }

        doc.version = id.version;
        doc.text = changes[0].text.clone();
        Ok(())
    }

    pub fn get(&self, url: Url) -> Option<&TextDocumentItem> {
        self.open_docs.get(&url)
    }

    pub fn get_mut(&mut self, url: Url) -> Option<&mut TextDocumentItem> {
        self.open_docs.get_mut(&url)
    }

    pub fn close(&mut self, url: Url) {
        self.open_docs.remove(&url);
    }
}
