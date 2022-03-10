#!/bin/bash
set -eu

rm test/data/*.gz
env RUST_BACKTRACE=1 ./zopfli test/data/*
mv test/data/*.gz test/results/
git diff --exit-code test/results/
