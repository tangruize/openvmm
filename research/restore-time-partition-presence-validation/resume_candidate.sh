#!/usr/bin/env bash
set -eu
cd "$(dirname "$0")/../.."
mv research/restore-time-partition-presence-validation/candidate.log \
    research/restore-time-partition-presence-validation/candidate-compile-import.log
bash research/restore-time-partition-presence-validation/launch_candidate.sh
