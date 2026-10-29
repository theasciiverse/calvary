#!/bin/sh
# Records the catalog preview, asciiverse/calvary.cast, to hub/RECORDING.md:
# 100x30, scripted with hub's tools/castrec.py. The opening's epigraph, the
# title over the globe, mission 1's briefing, then Al Basrah: the supertank
# drives up the road, locks on and shells the first turret, swings into
# the follow view, a look at the strategy map, and back out through the
# title. A build without gamepad support, so a pad on the recording
# machine cannot press anything, and a throwaway HOME, so no saved
# campaign progress is used or touched.
#
#   asciiverse/record-preview.sh
set -eu
cd "$(dirname "$0")/.."
cargo build --release --no-default-features --target-dir target/preview
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
script="w3;kspace;w3.5;kenter;w4.5;kspace;w2.5;kenter;w1.5;kw;kw;w5;kt;r6:space:1.4;w1;km;w3;r4:space:1.4;w1;ktab;w3;ktab;w1;kesc;w2;kesc;w1"
HOME=$tmp XDG_DATA_HOME=$tmp PATH="$PWD/target/preview/release:$PATH" \
  python3 ../../hub/tools/castrec.py asciiverse/calvary.cast "$tmp" "$script" -- calvary --mute
grep -l "/home/\|$(whoami)\|$(hostname)" asciiverse/calvary.cast && echo "personal data in the cast" && exit 1
ls -la asciiverse/calvary.cast
