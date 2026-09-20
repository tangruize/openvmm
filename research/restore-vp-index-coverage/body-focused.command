set -euo pipefail
git --no-pager diff -- vmm_core/src/partition_unit/vp_set.rs \
    > research/restore-vp-index-coverage/body-annotation.patch
timeout 110s bash research/restore-vp-index-coverage/intake-focused.command \
    > research/restore-vp-index-coverage/body-focused.log 2>&1
