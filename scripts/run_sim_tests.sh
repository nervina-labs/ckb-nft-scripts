#!/bin/bash
set -ex

# https://github.com/nervosnetwork/ckb-contract-guidelines/blob/main/rust/scripts/run_sim_tests.sh

ENVIRONMENT="$1"

SCRIPT_TOP="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"
TOP="$SCRIPT_TOP/.."

for i in $(find $TOP/build/$ENVIRONMENT/dumped_tests -mindepth 1 -maxdepth 1 -type d); do
    if [[ "$i" =~ ^.*_error.* ]]; then
        bash $i/cmd || error_code=$?
        if [[ "$error_code" -eq 0 ]]; then
           echo "Failure test passes!"
           exit 1
        fi
    else
        bash $i/cmd
    fi
done
