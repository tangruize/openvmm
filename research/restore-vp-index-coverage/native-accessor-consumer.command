set -euo pipefail
git --no-pager diff -- Cargo.lock vm/vmcore/vm_topology \
    vmm_core/src/lib.rs vmm_core/src/partition_unit/vp_set.rs \
    > research/restore-vp-index-coverage/native-accessor-consumer.patch
timeout 110s bash research/restore-vp-index-coverage/intake-focused.command \
    > research/restore-vp-index-coverage/native-accessor-consumer.log 2>&1
