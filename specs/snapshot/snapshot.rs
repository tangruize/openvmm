use vstd::prelude::*;

verus! {
    pub const MAX_SAVED_STATE_SIZE_BYTES: u64 = 256 * 1024 * 1024;

    pub struct SnapshotGenerationView {
        pub complete: bool,
        pub manifest_generation_id: u64,
        pub state_bytes: Seq<u8>,
        pub state_len: u64,
        pub memory_generation_id: u64,
        pub memory_len: u64,
        pub scratch_generation_id: u64,
        pub has_scratch: bool,
        pub intended_capture: bool,
    }

    pub struct WeakLengthOnlyGenerationView {
        pub complete: bool,
        pub state_len: u64,
        pub memory_len: u64,
        pub has_scratch: bool,
        pub intended_capture: bool,
    }

    pub struct RestoreOpenInput {
        pub directory_opened: bool,
        pub manifest_file_opened: bool,
        pub state_file_opened: bool,
        pub memory_file_opened: bool,
        pub manifest_decoded: bool,
        pub manifest_header_valid: bool,
        pub manifest_version_valid: bool,
        pub machine_contract_shape_valid: bool,
        pub directory_inventory_valid: bool,
        pub state_read_succeeded: bool,
        pub manifest_state_len: u64,
        pub observed_state_len: u64,
        pub state_bytes: Vec<u8>,
        pub manifest_memory_len: u64,
        pub observed_memory_len: u64,
        pub manifest_generation_id: u64,
        pub memory_generation_id: u64,
        pub scratch_generation_id: u64,
        pub has_scratch: bool,
        pub published_generation_belongs_to_capture: bool,
    }

    impl View for RestoreOpenInput {
        type V = SnapshotGenerationView;

        open spec fn view(&self) -> SnapshotGenerationView {
            SnapshotGenerationView {
                complete: restore_open_success(self),
                manifest_generation_id: self.manifest_generation_id,
                state_bytes: observed_state_bytes(self),
                state_len: if restore_open_success(self) {
                    self.observed_state_len
                } else {
                    0
                },
                memory_generation_id: self.memory_generation_id,
                memory_len: if restore_open_success(self) {
                    self.observed_memory_len
                } else {
                    0
                },
                scratch_generation_id: self.scratch_generation_id,
                has_scratch: self.has_scratch,
                intended_capture: restore_open_success(self)
                    && self.published_generation_belongs_to_capture,
            }
        }
    }

    pub struct WeakRestoreOpenInput {
        pub base: RestoreOpenInput,
    }

    impl View for WeakRestoreOpenInput {
        type V = WeakLengthOnlyGenerationView;

        open spec fn view(&self) -> WeakLengthOnlyGenerationView {
            weak_length_only_view(&self.base)
        }
    }

    pub struct RestoreAccessView {
        pub open_succeeded: bool,
        pub retained_directory_handle: bool,
        pub retained_manifest_handle: bool,
        pub retained_state_handle: bool,
        pub retained_memory_handle: bool,
        pub state_bytes_available: bool,
        pub state_bytes: Vec<u8>,
        pub state_len_matches_manifest: bool,
        pub memory_handle_available: bool,
        pub memory_generation_recorded: bool,
        pub memory_generation_id: u64,
        pub memory_len_matches_manifest: bool,
        pub intended_capture_guaranteed: bool,
    }

    pub enum StageSnapshotOutcomeView {
        Completed,
        FailedBeforeStagingOwner,
        RollbackSafeError,
        CleanupUncertainAliasError,
    }

    pub struct PublicationResultInput {
        pub stage: StageSnapshotOutcomeView,
        pub destination_absent_before_rename: bool,
        pub rename_committed: bool,
        pub rollback_cleanup_succeeded: bool,
        pub source_memory_alias: bool,
        pub parent_sync_succeeded: bool,
        pub generation: SnapshotGenerationView,
    }

    pub struct PublicationOutcomeView {
        pub report_success: bool,
        pub staging_or_rollback_failure: bool,
        pub rollback_safe: bool,
        pub cleanup_uncertain_alias: bool,
        pub committed_parent_sync_failure: bool,
        pub publication_committed: bool,
        pub complete_generation_available: bool,
        pub intended_capture_committed: bool,
    }

    pub open spec fn restore_open_success(input: &RestoreOpenInput) -> bool {
        input.directory_opened
            && input.manifest_file_opened
            && input.state_file_opened
            && input.memory_file_opened
            && input.manifest_decoded
            && input.manifest_header_valid
            && input.manifest_version_valid
            && input.machine_contract_shape_valid
            && input.directory_inventory_valid
            && input.manifest_state_len <= MAX_SAVED_STATE_SIZE_BYTES
            && input.state_read_succeeded
            && input.state_bytes@.len() == input.observed_state_len as nat
            && input.observed_state_len == input.manifest_state_len
            && input.observed_memory_len == input.manifest_memory_len
    }

    pub open spec fn observed_state_bytes(input: &RestoreOpenInput) -> Seq<u8> {
        if restore_open_success(input) {
            input.state_bytes@
        } else {
            Seq::<u8>::empty()
        }
    }

    pub open spec fn weak_length_only_view(
        input: &RestoreOpenInput,
    ) -> WeakLengthOnlyGenerationView {
        WeakLengthOnlyGenerationView {
            complete: restore_open_success(input),
            state_len: if restore_open_success(input) {
                input.observed_state_len
            } else {
                0
            },
            memory_len: if restore_open_success(input) {
                input.observed_memory_len
            } else {
                0
            },
            has_scratch: input.has_scratch,
            intended_capture: restore_open_success(input)
                && input.published_generation_belongs_to_capture,
        }
    }

    pub open spec fn access_state_bytes_from_generation(
        view: SnapshotGenerationView,
    ) -> Seq<u8> {
        if view.complete {
            view.state_bytes
        } else {
            Seq::<u8>::empty()
        }
    }

    pub open spec fn access_state_len_matches_manifest(view: SnapshotGenerationView) -> bool {
        view.complete && view.state_bytes.len() == view.state_len as nat
    }

    pub open spec fn access_memory_generation_id(view: SnapshotGenerationView) -> u64 {
        if view.complete {
            view.memory_generation_id
        } else {
            0
        }
    }

    pub fn opened_generation_access_view(opened: &RestoreOpenInput) -> (view: RestoreAccessView)
        ensures
            view.open_succeeded == opened@.complete,
            view.retained_directory_handle == opened@.complete,
            view.retained_manifest_handle == opened@.complete,
            view.retained_state_handle == opened@.complete,
            view.retained_memory_handle == opened@.complete,
            view.state_bytes_available == opened@.complete,
            view.state_bytes@ == access_state_bytes_from_generation(opened@),
            view.state_len_matches_manifest == access_state_len_matches_manifest(opened@),
            view.memory_handle_available == opened@.complete,
            view.memory_generation_recorded == opened@.complete,
            view.memory_generation_id == access_memory_generation_id(opened@),
            view.memory_len_matches_manifest == opened@.complete,
            view.intended_capture_guaranteed == opened@.intended_capture,
    {
        let ok = opened.directory_opened
            && opened.manifest_file_opened
            && opened.state_file_opened
            && opened.memory_file_opened
            && opened.manifest_decoded
            && opened.manifest_header_valid
            && opened.manifest_version_valid
            && opened.machine_contract_shape_valid
            && opened.directory_inventory_valid
            && opened.manifest_state_len <= MAX_SAVED_STATE_SIZE_BYTES
            && opened.state_read_succeeded
            && opened.state_bytes.len() as u64 == opened.observed_state_len
            && opened.observed_state_len == opened.manifest_state_len
            && opened.observed_memory_len == opened.manifest_memory_len;
        RestoreAccessView {
            open_succeeded: ok,
            retained_directory_handle: ok,
            retained_manifest_handle: ok,
            retained_state_handle: ok,
            retained_memory_handle: ok,
            state_bytes_available: ok,
            state_bytes: if ok { opened.state_bytes.clone() } else { Vec::new() },
            state_len_matches_manifest: ok,
            memory_handle_available: ok,
            memory_generation_recorded: ok,
            memory_generation_id: if ok { opened.memory_generation_id } else { 0 },
            memory_len_matches_manifest: ok,
            intended_capture_guaranteed: ok && opened.published_generation_belongs_to_capture,
        }
    }

    pub fn weak_length_only_generation_access_view(
        opened: &WeakRestoreOpenInput,
    ) -> (view: RestoreAccessView)
        ensures
            view.open_succeeded == opened.base@.complete,
            view.retained_directory_handle == opened.base@.complete,
            view.retained_manifest_handle == opened.base@.complete,
            view.retained_state_handle == opened.base@.complete,
            view.retained_memory_handle == opened.base@.complete,
            view.state_bytes_available == opened.base@.complete,
            view.state_bytes@ == access_state_bytes_from_generation(opened.base@),
            view.state_len_matches_manifest == access_state_len_matches_manifest(opened.base@),
            view.memory_handle_available == opened.base@.complete,
            view.memory_generation_recorded == opened.base@.complete,
            view.memory_generation_id == access_memory_generation_id(opened.base@),
            view.memory_len_matches_manifest == opened.base@.complete,
            view.intended_capture_guaranteed == opened.base@.intended_capture,
    {
        opened_generation_access_view(&opened.base)
    }

    pub fn same_length_state_substitution_witness_pair() -> (pair: (RestoreOpenInput, RestoreOpenInput))
        ensures
            restore_open_success(&pair.0),
            restore_open_success(&pair.1),
            weak_length_only_view(&pair.0) == weak_length_only_view(&pair.1),
            pair.0@.state_bytes != pair.1@.state_bytes,
            access_state_bytes_from_generation(pair.0@) != access_state_bytes_from_generation(pair.1@),
    {
        let first = RestoreOpenInput {
            directory_opened: true,
            manifest_file_opened: true,
            state_file_opened: true,
            memory_file_opened: true,
            manifest_decoded: true,
            manifest_header_valid: true,
            manifest_version_valid: true,
            machine_contract_shape_valid: true,
            directory_inventory_valid: true,
            state_read_succeeded: true,
            manifest_state_len: 1,
            observed_state_len: 1,
            state_bytes: vec![1u8],
            manifest_memory_len: 4096,
            observed_memory_len: 4096,
            manifest_generation_id: 7,
            memory_generation_id: 11,
            scratch_generation_id: 0,
            has_scratch: false,
            published_generation_belongs_to_capture: true,
        };
        let second = RestoreOpenInput {
            directory_opened: true,
            manifest_file_opened: true,
            state_file_opened: true,
            memory_file_opened: true,
            manifest_decoded: true,
            manifest_header_valid: true,
            manifest_version_valid: true,
            machine_contract_shape_valid: true,
            directory_inventory_valid: true,
            state_read_succeeded: true,
            manifest_state_len: 1,
            observed_state_len: 1,
            state_bytes: vec![2u8],
            manifest_memory_len: 4096,
            observed_memory_len: 4096,
            manifest_generation_id: 7,
            memory_generation_id: 11,
            scratch_generation_id: 0,
            has_scratch: false,
            published_generation_belongs_to_capture: true,
        };
        (first, second)
    }

    pub proof fn weak_length_only_view_misses_same_length_state_substitution(
        first: &RestoreOpenInput,
        second: &RestoreOpenInput,
    )
        requires
            restore_open_success(first),
            restore_open_success(second),
            first.observed_state_len == second.observed_state_len,
            first.observed_memory_len == second.observed_memory_len,
            first.has_scratch == second.has_scratch,
            first.published_generation_belongs_to_capture
                == second.published_generation_belongs_to_capture,
            first.state_bytes@ != second.state_bytes@,
        ensures
            weak_length_only_view(first) == weak_length_only_view(second),
            first@.state_bytes != second@.state_bytes,
            access_state_bytes_from_generation(first@) != access_state_bytes_from_generation(second@),
    {
    }

    pub open spec fn stage_snapshot_completed(input: &PublicationResultInput) -> bool {
        input.stage is Completed
    }

    pub open spec fn pre_commit_failure_after_staging(input: &PublicationResultInput) -> bool {
        stage_snapshot_completed(input)
            && (!input.destination_absent_before_rename || !input.rename_committed)
    }

    pub open spec fn rollback_cleanup_uncertain(input: &PublicationResultInput) -> bool {
        input.source_memory_alias && !input.rollback_cleanup_succeeded
    }

    pub open spec fn stage_failure_cleanup_uncertain(input: &PublicationResultInput) -> bool {
        input.stage is CleanupUncertainAliasError
    }

    pub open spec fn stage_failure_rollback_safe(input: &PublicationResultInput) -> bool {
        input.stage is FailedBeforeStagingOwner || input.stage is RollbackSafeError
    }

    pub open spec fn retired_publication_outcome_stage_failure_boolean(
        input: &PublicationResultInput,
    ) -> PublicationOutcomeView {
        if !stage_snapshot_completed(input) {
            PublicationOutcomeView {
                report_success: false,
                staging_or_rollback_failure: true,
                rollback_safe: true,
                cleanup_uncertain_alias: false,
                committed_parent_sync_failure: false,
                publication_committed: false,
                complete_generation_available: false,
                intended_capture_committed: false,
            }
        } else if pre_commit_failure_after_staging(input) && rollback_cleanup_uncertain(input) {
            PublicationOutcomeView {
                report_success: false,
                staging_or_rollback_failure: true,
                rollback_safe: false,
                cleanup_uncertain_alias: true,
                committed_parent_sync_failure: false,
                publication_committed: false,
                complete_generation_available: false,
                intended_capture_committed: false,
            }
        } else if pre_commit_failure_after_staging(input) {
            PublicationOutcomeView {
                report_success: false,
                staging_or_rollback_failure: true,
                rollback_safe: true,
                cleanup_uncertain_alias: false,
                committed_parent_sync_failure: false,
                publication_committed: false,
                complete_generation_available: false,
                intended_capture_committed: false,
            }
        } else if !input.parent_sync_succeeded {
            PublicationOutcomeView {
                report_success: false,
                staging_or_rollback_failure: false,
                rollback_safe: false,
                cleanup_uncertain_alias: false,
                committed_parent_sync_failure: true,
                publication_committed: true,
                complete_generation_available: input.generation.complete,
                intended_capture_committed: input.generation.complete
                    && input.generation.intended_capture,
            }
        } else {
            PublicationOutcomeView {
                report_success: input.rename_committed,
                staging_or_rollback_failure: !input.rename_committed,
                rollback_safe: !input.rename_committed,
                cleanup_uncertain_alias: false,
                committed_parent_sync_failure: false,
                publication_committed: input.rename_committed,
                complete_generation_available: input.rename_committed && input.generation.complete,
                intended_capture_committed: input.rename_committed
                    && input.generation.complete
                    && input.generation.intended_capture,
            }
        }
    }

    pub open spec fn retired_publication_outcome_destination_absence_first(
        input: &PublicationResultInput,
    ) -> PublicationOutcomeView {
        if !stage_snapshot_completed(input)
            || !input.destination_absent_before_rename
            || (!input.rename_committed && input.rollback_cleanup_succeeded)
        {
            PublicationOutcomeView {
                report_success: false,
                staging_or_rollback_failure: true,
                rollback_safe: true,
                cleanup_uncertain_alias: false,
                committed_parent_sync_failure: false,
                publication_committed: false,
                complete_generation_available: false,
                intended_capture_committed: false,
            }
        } else if !input.rename_committed && input.source_memory_alias {
            PublicationOutcomeView {
                report_success: false,
                staging_or_rollback_failure: true,
                rollback_safe: false,
                cleanup_uncertain_alias: true,
                committed_parent_sync_failure: false,
                publication_committed: false,
                complete_generation_available: false,
                intended_capture_committed: false,
            }
        } else if input.rename_committed && !input.parent_sync_succeeded {
            PublicationOutcomeView {
                report_success: false,
                staging_or_rollback_failure: false,
                rollback_safe: false,
                cleanup_uncertain_alias: false,
                committed_parent_sync_failure: true,
                publication_committed: true,
                complete_generation_available: input.generation.complete,
                intended_capture_committed: input.generation.complete
                    && input.generation.intended_capture,
            }
        } else {
            PublicationOutcomeView {
                report_success: input.rename_committed,
                staging_or_rollback_failure: !input.rename_committed,
                rollback_safe: !input.rename_committed,
                cleanup_uncertain_alias: false,
                committed_parent_sync_failure: false,
                publication_committed: input.rename_committed,
                complete_generation_available: input.rename_committed && input.generation.complete,
                intended_capture_committed: input.rename_committed
                    && input.generation.complete
                    && input.generation.intended_capture,
            }
        }
    }

    pub open spec fn publication_outcome(input: &PublicationResultInput) -> PublicationOutcomeView {
        if stage_failure_cleanup_uncertain(input) {
            PublicationOutcomeView {
                report_success: false,
                staging_or_rollback_failure: true,
                rollback_safe: false,
                cleanup_uncertain_alias: true,
                committed_parent_sync_failure: false,
                publication_committed: false,
                complete_generation_available: false,
                intended_capture_committed: false,
            }
        } else if stage_failure_rollback_safe(input) {
            PublicationOutcomeView {
                report_success: false,
                staging_or_rollback_failure: true,
                rollback_safe: true,
                cleanup_uncertain_alias: false,
                committed_parent_sync_failure: false,
                publication_committed: false,
                complete_generation_available: false,
                intended_capture_committed: false,
            }
        } else if pre_commit_failure_after_staging(input) && rollback_cleanup_uncertain(input) {
            PublicationOutcomeView {
                report_success: false,
                staging_or_rollback_failure: true,
                rollback_safe: false,
                cleanup_uncertain_alias: true,
                committed_parent_sync_failure: false,
                publication_committed: false,
                complete_generation_available: false,
                intended_capture_committed: false,
            }
        } else if pre_commit_failure_after_staging(input) {
            PublicationOutcomeView {
                report_success: false,
                staging_or_rollback_failure: true,
                rollback_safe: true,
                cleanup_uncertain_alias: false,
                committed_parent_sync_failure: false,
                publication_committed: false,
                complete_generation_available: false,
                intended_capture_committed: false,
            }
        } else if !input.parent_sync_succeeded {
            PublicationOutcomeView {
                report_success: false,
                staging_or_rollback_failure: false,
                rollback_safe: false,
                cleanup_uncertain_alias: false,
                committed_parent_sync_failure: true,
                publication_committed: true,
                complete_generation_available: input.generation.complete,
                intended_capture_committed: input.generation.complete
                    && input.generation.intended_capture,
            }
        } else {
            PublicationOutcomeView {
                report_success: input.rename_committed,
                staging_or_rollback_failure: !input.rename_committed,
                rollback_safe: !input.rename_committed,
                cleanup_uncertain_alias: false,
                committed_parent_sync_failure: false,
                publication_committed: input.rename_committed,
                complete_generation_available: input.rename_committed && input.generation.complete,
                intended_capture_committed: input.rename_committed
                    && input.generation.complete
                    && input.generation.intended_capture,
            }
        }
    }

    pub open spec fn complete_generation_witness() -> SnapshotGenerationView {
        SnapshotGenerationView {
            complete: true,
            manifest_generation_id: 7,
            state_bytes: seq![1u8],
            state_len: 1,
            memory_generation_id: 11,
            memory_len: 4096,
            scratch_generation_id: 0,
            has_scratch: false,
            intended_capture: true,
        }
    }

    pub open spec fn stage_cleanup_uncertain_failure_witness() -> PublicationResultInput {
        PublicationResultInput {
            stage: StageSnapshotOutcomeView::CleanupUncertainAliasError,
            destination_absent_before_rename: true,
            rename_committed: false,
            rollback_cleanup_succeeded: false,
            source_memory_alias: true,
            parent_sync_succeeded: false,
            generation: complete_generation_witness(),
        }
    }

    pub proof fn stage_internal_cleanup_failure_distinguishes_publication_outcome_repair()
        ensures
            stage_cleanup_uncertain_failure_witness().stage is CleanupUncertainAliasError,
            retired_publication_outcome_stage_failure_boolean(
                &stage_cleanup_uncertain_failure_witness(),
            ).rollback_safe,
            !retired_publication_outcome_stage_failure_boolean(
                &stage_cleanup_uncertain_failure_witness(),
            ).cleanup_uncertain_alias,
            publication_outcome(&stage_cleanup_uncertain_failure_witness()).cleanup_uncertain_alias,
            !publication_outcome(&stage_cleanup_uncertain_failure_witness()).rollback_safe,
            publication_outcome(&stage_cleanup_uncertain_failure_witness()).staging_or_rollback_failure,
            !publication_outcome(&stage_cleanup_uncertain_failure_witness()).publication_committed,
    {
    }

    pub proof fn publication_preserves_every_stage_failure(input: &PublicationResultInput)
        ensures
            !stage_snapshot_completed(input) ==> !publication_outcome(input).publication_committed,
            !stage_snapshot_completed(input) ==> !publication_outcome(input).report_success,
            stage_failure_cleanup_uncertain(input) ==> publication_outcome(input).cleanup_uncertain_alias,
            stage_failure_cleanup_uncertain(input) ==> !publication_outcome(input).rollback_safe,
            stage_failure_rollback_safe(input) ==> publication_outcome(input).rollback_safe,
            stage_failure_rollback_safe(input) ==> !publication_outcome(input).cleanup_uncertain_alias,
    {
    }

    pub open spec fn destination_absence_cleanup_uncertain_witness() -> PublicationResultInput {
        PublicationResultInput {
            stage: StageSnapshotOutcomeView::Completed,
            destination_absent_before_rename: false,
            rename_committed: false,
            rollback_cleanup_succeeded: false,
            source_memory_alias: true,
            parent_sync_succeeded: false,
            generation: complete_generation_witness(),
        }
    }

    pub proof fn destination_absence_alias_cleanup_failure_distinguishes_publication_outcome_repair()
        ensures
            destination_absence_cleanup_uncertain_witness().stage is Completed,
            retired_publication_outcome_destination_absence_first(
                &destination_absence_cleanup_uncertain_witness(),
            ).rollback_safe,
            !retired_publication_outcome_destination_absence_first(
                &destination_absence_cleanup_uncertain_witness(),
            ).cleanup_uncertain_alias,
            publication_outcome(&destination_absence_cleanup_uncertain_witness()).cleanup_uncertain_alias,
            !publication_outcome(&destination_absence_cleanup_uncertain_witness()).rollback_safe,
            publication_outcome(&destination_absence_cleanup_uncertain_witness()).staging_or_rollback_failure,
            !publication_outcome(&destination_absence_cleanup_uncertain_witness()).publication_committed,
    {
    }
}

fn main() {}
