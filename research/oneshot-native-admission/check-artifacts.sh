#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../.."
exec > research/oneshot-native-admission/artifact-integrity.log 2>&1
set -x
git diff --exit-code -- . ':!research/**' ':!.autors/**'
git diff --cached --exit-code -- . ':!research/**' ':!.autors/**'
git diff --check
git apply --check research/oneshot-native-admission/receiver-source.patch
git apply --check research/oneshot-native-admission/representation-source.patch
bash -n research/oneshot-native-admission/intake.sh \
    research/oneshot-native-admission/verify-owner.sh \
    research/oneshot-native-admission/reproduce.sh \
    research/oneshot-native-admission/probe.sh \
    research/oneshot-native-admission/check.sh
git --no-pager status --short
git --no-pager log -1 --format='%h %cI %s'
git --no-pager log -1 --format='%h %cI %s' argus/restore-v1-frozen
printf '%s\n' 'production_source_and_manifests_restored=true'
