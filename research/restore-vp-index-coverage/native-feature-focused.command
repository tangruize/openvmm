set -euo pipefail
git --no-pager diff -- vmm_core/src/lib.rs vmm_core/src/partition_unit/vp_set.rs \
    > research/restore-vp-index-coverage/native-feature-annotation.patch
timeout 110s bash research/restore-vp-index-coverage/intake-focused.command \
    > research/restore-vp-index-coverage/native-feature-focused.log 2>&1
