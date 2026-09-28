use vstd::prelude::*;

#[macro_export]
macro_rules! __snapshot_proof_tracing_warn {
    ($($tt:tt)*) => {{}};
}

verus! {
    pub mod tracing {
        pub use crate::__snapshot_proof_tracing_warn as warn;
    }

    pub mod anyhow {
        pub type Error = super::Error;
    }

    pub const MAX_SAVED_STATE_SIZE_BYTES: u64 = 256 * 1024 * 1024;

    pub struct RestoreOpenInputs {
        pub directory_opened: bool,
        pub manifest_file_opened: bool,
        pub state_file_opened: bool,
        pub memory_file_opened: bool,
        pub manifest_decoded: bool,
        pub manifest_header_valid: bool,
        pub manifest_version_valid: bool,
        pub machine_contract_shape_valid: bool,
        pub directory_inventory_valid: bool,
        pub state_manifest_size: u64,
        pub state_read_succeeded: bool,
        pub state_len: u64,
        pub memory_generation_observed: bool,
        pub memory_generation_len: u64,
        pub manifest_memory_size: u64,
        pub published_generation_belongs_to_capture: bool,
    }

    pub struct RestoreOpenView {
        pub open_succeeded: bool,
        pub retained_directory_handle: bool,
        pub retained_manifest_handle: bool,
        pub retained_state_handle: bool,
        pub retained_memory_handle: bool,
        pub directory_inventory_valid: bool,
        pub state_bytes_available: bool,
        pub state_len_matches_manifest: bool,
        pub memory_handle_available: bool,
        pub memory_generation_recorded: bool,
        pub memory_len_matches_manifest: bool,
        pub intended_capture_guaranteed: bool,
    }

    pub open spec fn opened_snapshot_open_success(input: RestoreOpenInputs) -> bool {
        input.directory_opened
            && input.manifest_file_opened
            && input.state_file_opened
            && input.memory_file_opened
            && input.manifest_decoded
            && input.manifest_header_valid
            && input.manifest_version_valid
            && input.machine_contract_shape_valid
            && input.directory_inventory_valid
            && input.state_manifest_size <= MAX_SAVED_STATE_SIZE_BYTES
            && input.state_read_succeeded
            && input.state_len == input.state_manifest_size
            && input.memory_generation_observed
            && input.memory_generation_len == input.manifest_memory_size
    }

    pub fn opened_snapshot_open_view(input: RestoreOpenInputs) -> (view: RestoreOpenView)
        ensures
            view.open_succeeded == opened_snapshot_open_success(input),
            view.retained_directory_handle == opened_snapshot_open_success(input),
            view.retained_manifest_handle == opened_snapshot_open_success(input),
            view.retained_state_handle == opened_snapshot_open_success(input),
            view.retained_memory_handle == opened_snapshot_open_success(input),
            view.directory_inventory_valid == opened_snapshot_open_success(input),
            view.state_bytes_available == opened_snapshot_open_success(input),
            view.state_len_matches_manifest == opened_snapshot_open_success(input),
            view.memory_handle_available == opened_snapshot_open_success(input),
            view.memory_generation_recorded == opened_snapshot_open_success(input),
            view.memory_len_matches_manifest == opened_snapshot_open_success(input),
            view.intended_capture_guaranteed == (
                opened_snapshot_open_success(input)
                    && input.published_generation_belongs_to_capture
            ),
    {
        let ok = input.directory_opened
            && input.manifest_file_opened
            && input.state_file_opened
            && input.memory_file_opened
            && input.manifest_decoded
            && input.manifest_header_valid
            && input.manifest_version_valid
            && input.machine_contract_shape_valid
            && input.directory_inventory_valid
            && input.state_manifest_size <= MAX_SAVED_STATE_SIZE_BYTES
            && input.state_read_succeeded
            && input.state_len == input.state_manifest_size
            && input.memory_generation_observed
            && input.memory_generation_len == input.manifest_memory_size;

        RestoreOpenView {
            open_succeeded: ok,
            retained_directory_handle: ok,
            retained_manifest_handle: ok,
            retained_state_handle: ok,
            retained_memory_handle: ok,
            directory_inventory_valid: ok,
            state_bytes_available: ok,
            state_len_matches_manifest: ok,
            memory_handle_available: ok,
            memory_generation_recorded: ok,
            memory_len_matches_manifest: ok,
            intended_capture_guaranteed: ok && input.published_generation_belongs_to_capture,
        }
    }

    pub mod proof_std {
        pub mod fs {
            pub struct File {}
        }
    }

    use self::proof_std as std;

    pub struct Path {}

    #[derive(Clone)]
    pub struct PathBuf {}

    pub struct Error {}

    impl Path {
        #[verifier::external_body]
        pub fn to_owned(&self) -> PathBuf {
            PathBuf {}
        }

        #[verifier::external_body]
        pub fn display(&self) -> &str {
            ""
        }
    }

    pub struct SnapshotManifest {}

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub enum MemoryPublication {
        IndependentCopy,
        OwnedExactFile,
    }

    pub enum SnapshotWriteError {
        BeforeCommit(Error),
        CleanupUncertain {
            path: PathBuf,
            error: Error,
            cleanup_error: Error,
        },
        Committed {
            path: PathBuf,
            error: Error,
        },
    }

    pub struct PublicationView {
        pub report_success: bool,
        pub staging_or_rollback_failure: bool,
        pub rollback_safe: bool,
        pub cleanup_uncertain_alias: bool,
        pub committed_parent_sync_failure: bool,
        pub publication_committed: bool,
        pub complete_generation_available: bool,
    }

    pub open spec fn publication_view(result: Result<(), SnapshotWriteError>) -> PublicationView {
        match result {
            Ok(()) => PublicationView {
                report_success: true,
                staging_or_rollback_failure: false,
                rollback_safe: false,
                cleanup_uncertain_alias: false,
                committed_parent_sync_failure: false,
                publication_committed: true,
                complete_generation_available: true,
            },
            Err(SnapshotWriteError::Committed { path: _, error: _ }) => PublicationView {
                report_success: false,
                staging_or_rollback_failure: false,
                rollback_safe: false,
                cleanup_uncertain_alias: false,
                committed_parent_sync_failure: true,
                publication_committed: true,
                complete_generation_available: true,
            },
            Err(SnapshotWriteError::BeforeCommit(_)) => PublicationView {
                report_success: false,
                staging_or_rollback_failure: true,
                rollback_safe: true,
                cleanup_uncertain_alias: false,
                committed_parent_sync_failure: false,
                publication_committed: false,
                complete_generation_available: false,
            },
            Err(SnapshotWriteError::CleanupUncertain { path: _, error: _, cleanup_error: _ }) => {
                PublicationView {
                    report_success: false,
                    staging_or_rollback_failure: true,
                    rollback_safe: false,
                    cleanup_uncertain_alias: true,
                    committed_parent_sync_failure: false,
                    publication_committed: false,
                    complete_generation_available: false,
                }
            },
        }
    }

    pub open spec fn publication_result_classified(
        result: Result<(), SnapshotWriteError>,
    ) -> bool {
        let view = publication_view(result);
        (view.report_success
            && view.publication_committed
            && view.complete_generation_available)
            || (view.committed_parent_sync_failure
                && view.publication_committed
                && view.complete_generation_available)
            || (view.staging_or_rollback_failure
                && view.rollback_safe
                && !view.publication_committed)
            || (view.staging_or_rollback_failure
                && view.cleanup_uncertain_alias
                && !view.publication_committed)
    }

    pub struct RollbackClassificationView {
        pub before_commit: bool,
        pub cleanup_uncertain_alias: bool,
        pub rollback_safe: bool,
        pub publication_committed: bool,
        pub final_destination_published: bool,
    }

    pub open spec fn rollback_classification_view(
        result: SnapshotWriteError,
    ) -> RollbackClassificationView {
        match result {
            SnapshotWriteError::BeforeCommit(_) => RollbackClassificationView {
                before_commit: true,
                cleanup_uncertain_alias: false,
                rollback_safe: true,
                publication_committed: false,
                final_destination_published: false,
            },
            SnapshotWriteError::CleanupUncertain { path: _, error: _, cleanup_error: _ } => {
                RollbackClassificationView {
                    before_commit: false,
                    cleanup_uncertain_alias: true,
                    rollback_safe: false,
                    publication_committed: false,
                    final_destination_published: false,
                }
            },
            SnapshotWriteError::Committed { path: _, error: _ } => RollbackClassificationView {
                before_commit: false,
                cleanup_uncertain_alias: false,
                rollback_safe: false,
                publication_committed: true,
                final_destination_published: true,
            },
        }
    }

    pub struct StagingDirectory {
        pub path: Option<PathBuf>,
        pub path_ref: Path,
        pub source_memory_alias: bool,
    }

    pub struct StagingCommitView {
        pub publish_succeeded: bool,
        pub rename_committed: bool,
        pub staging_owner_retired: bool,
    }

    pub open spec fn staging_commit_view(result: Result<(), Error>) -> StagingCommitView {
        match result {
            Ok(()) => StagingCommitView {
                publish_succeeded: true,
                rename_committed: true,
                staging_owner_retired: true,
            },
            Err(_) => StagingCommitView {
                publish_succeeded: false,
                rename_committed: false,
                staging_owner_retired: false,
            },
        }
    }

    pub struct RenameNoReplaceView {
        pub returned_success: bool,
        pub committed_without_replace: bool,
        pub failed_before_commit: bool,
        pub environment_dependent: bool,
    }

    pub open spec fn rename_no_replace_view(result: Result<(), Error>) -> RenameNoReplaceView {
        match result {
            Ok(()) => RenameNoReplaceView {
                returned_success: true,
                committed_without_replace: true,
                failed_before_commit: false,
                environment_dependent: true,
            },
            Err(_) => RenameNoReplaceView {
                returned_success: false,
                committed_without_replace: false,
                failed_before_commit: true,
                environment_dependent: true,
            },
        }
    }

    pub trait Context<T> {
        fn with_context<C, F: FnOnce() -> C>(self, f: F) -> Result<T, Error>;
    }

    impl<T> Context<T> for Result<T, Error> {
        #[verifier::external_body]
        fn with_context<C, F: FnOnce() -> C>(self, f: F) -> (result: Result<T, Error>)
            ensures
                result is Ok <==> self is Ok,
                result is Err <==> self is Err,
        {
            self
        }
    }

    #[verifier::external_body]
    pub fn rename_no_replace(source: &Path, destination: &Path) -> (result: Result<(), Error>)
        ensures
            rename_no_replace_view(result).returned_success == (result is Ok),
            rename_no_replace_view(result).committed_without_replace == (result is Ok),
            rename_no_replace_view(result).failed_before_commit == (result is Err),
            rename_no_replace_view(result).environment_dependent,
    {
        Ok(())
    }

    impl StagingDirectory {
        pub fn path(&self) -> &Path {
            &self.path_ref
        }

        pub fn publish(&mut self, destination: &Path) -> (result: Result<(), Error>)
            ensures
                staging_commit_view(result).publish_succeeded
                    == staging_commit_view(result).rename_committed,
                staging_commit_view(result).publish_succeeded
                    == staging_commit_view(result).staging_owner_retired,
                result is Ok ==> final(self).path is None,
                result is Err ==> final(self).path == old(self).path,
                final(self).path_ref == old(self).path_ref,
                final(self).source_memory_alias == old(self).source_memory_alias,
        {
            let staging_path = self.path();
            rename_no_replace(staging_path, destination).with_context(|| {
                format!(
                    "failed to publish snapshot {} to {}",
                    staging_path.display(),
                    destination.display()
                )
            })?;
            self.path = None;
            Ok(())
        }

        #[verifier::external_body]
        pub fn remove_staging_directory(&mut self) -> (result: Result<(), Error>)
            ensures
                result is Ok ==> final(self).path is None,
                result is Ok && old(self).path is Some ==> !final(self).source_memory_alias,
                result is Err ==> final(self).path == old(self).path,
                result is Err ==> final(self).source_memory_alias == old(self).source_memory_alias,
                final(self).path_ref == old(self).path_ref,
        {
            self.path = None;
            self.source_memory_alias = false;
            Ok(())
        }

        pub fn rollback(mut self, error: anyhow::Error) -> (result: SnapshotWriteError)
            requires
                self.path is Some,
            ensures
                rollback_classification_view(result).publication_committed == false,
                rollback_classification_view(result).final_destination_published == false,
                rollback_classification_view(result).before_commit
                    || rollback_classification_view(result).cleanup_uncertain_alias,
                result is BeforeCommit ==> rollback_classification_view(result).rollback_safe,
                result is CleanupUncertain ==>
                    rollback_classification_view(result).cleanup_uncertain_alias,
        {
            if self.source_memory_alias
                && let Err(cleanup_error) = self.remove_staging_directory()
            {
                return SnapshotWriteError::CleanupUncertain {
                    path: self
                        .path
                        .clone()
                        .expect("unpublished staging path is present"),
                    error,
                    cleanup_error,
                };
            }

            if let Err(cleanup_error) = self.remove_staging_directory() {
                tracing::warn!(
                    error = cleanup_error.as_ref() as &dyn std::error::Error,
                    "failed to remove independent snapshot staging artifacts"
                );
            }
            SnapshotWriteError::BeforeCommit(error)
        }
    }

    #[verifier::external_body]
    pub fn stage_snapshot(
        dir: &Path,
        manifest: &SnapshotManifest,
        saved_state_bytes: &[u8],
        memory_file: &std::fs::File,
        scratch_file: Option<&std::fs::File>,
        memory_publication: MemoryPublication,
    ) -> Result<StagingDirectory, SnapshotWriteError> {
        Err(SnapshotWriteError::BeforeCommit(Error {}))
    }

    #[verifier::external_body]
    pub fn ensure_path_absent(dir: &Path, description: &str) -> Result<(), Error> {
        Ok(())
    }

    #[verifier::external_body]
    pub fn sync_directory(dir: &Path) -> Result<(), Error> {
        Ok(())
    }

    pub fn snapshot_parent(dir: &Path) -> &Path {
        dir
    }

    pub mod openvmm_defs {
        pub mod profile {
            pub struct ProfileSpan {}

            impl ProfileSpan {
                pub fn start() -> Self {
                    ProfileSpan {}
                }

                pub fn complete(self, category: &str, name: &str, counters: ()) {}
            }
        }
    }
}
