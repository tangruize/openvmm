set -eu
timeout 110s "${ARGUS_SKILL_PYTHON:-python3}" -m argus_verus.tools.source.callers \
    vmm_core/src/partition_unit/vp_set.rs --project-dir . --markdown
