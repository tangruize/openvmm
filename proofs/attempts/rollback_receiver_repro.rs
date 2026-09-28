use vstd::prelude::*;

// Reduced receiver/let-chain reproducer with stub types and omitted logging, not a source-body proof.
verus! {
    pub mod anyhow {
        pub type Error = super::Error;
    }

    pub struct Error {}

    #[derive(Clone)]
    pub struct PathBuf {}

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

    pub struct StagingDirectory {
        pub path: Option<PathBuf>,
        pub source_memory_alias: bool,
    }

    impl StagingDirectory {
        #[verifier::external_body]
        pub fn remove_staging_directory(&mut self) -> Result<(), Error> {
            Ok(())
        }

        pub fn rollback(mut self, error: anyhow::Error) -> SnapshotWriteError
            requires
                self.path is Some,
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
                let _ = cleanup_error;
            }
            SnapshotWriteError::BeforeCommit(error)
        }
    }
}

fn main() {}
