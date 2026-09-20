bash -c verification/tools/find-verus.sh\;\ toolchain/verus-src/source/z3\ --version\;\ printf\ \"rustup=%s\\n\"\ \"\$\{RUSTUP_TOOLCHAIN:-unset\}\"\;\ test\ -x\ \"\$\{ARGUS_SKILL_PYTHON:-python3\}\" 
