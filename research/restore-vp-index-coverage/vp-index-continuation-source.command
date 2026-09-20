set -eu
git --no-pager diff -- Cargo.lock vm/vmcore/vm_topology/Cargo.toml \
    vm/vmcore/vm_topology/src/processor.rs
git --no-pager status --short
