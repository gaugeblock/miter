//! The per-file baseline the incremental passes depend on (`GB-302`).
//!
//! Split from `store/mod.rs` when the accessors pushed that file past the 250-line limit. The field
//! itself stays on `Engine`; only its surface lives here.

use super::Engine;

impl Engine {
    /// Records the baseline row for one file.
    pub fn record_file(&mut self, relative: String, record: miter_storage::IndexedFileRecord) {
        self.file_ledger.insert(relative, record);
    }

    /// Mutable access for the ingest passes that fill the baseline.
    pub(crate) fn file_ledger_mut(
        &mut self,
    ) -> &mut std::collections::BTreeMap<String, miter_storage::IndexedFileRecord> {
        &mut self.file_ledger
    }

    /// The per-file baseline, for a persister that must write it back.
    #[must_use]
    pub fn file_ledger(
        &self,
    ) -> &std::collections::BTreeMap<String, miter_storage::IndexedFileRecord> {
        &self.file_ledger
    }
}
