#!/usr/bin/env bash

set -e -x

pushd doc/

sed -i -r -e 's/sha256-[a-zA-Z0-9+]+=/sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=/g' package.nix

NEW="$(nix build .#docs 2>&1 | sed -n -r 's/got:\s+(sha256-[a-zA-Z0-9+]+=)/\1/p' | awk '{$1=$1;print}')"

echo "new: $NEW"

sed -i -r -e "s/sha256-[a-zA-Z0-9+]+=/$NEW/g" package.nix
