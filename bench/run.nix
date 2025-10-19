{ inputs, ... }:
{
  perSystem =
    {
      pkgs,
      lib,
      self',
      ...
    }:
    {
      packages = {
        bench-runner = pkgs.writeShellScriptBin "bench-runner" ''
          set -e

          export NIX_PATH="nixpkgs=${inputs.nixpkgs}"

          setup_vm() {
            local i="$1"

            echo "building $i"

            out=$(INDEX="$i" nix-build '<nixpkgs/nixos>' -A vm -I nixos-config=./vm.nix --no-link)

            echo "$out/bin/run-bench-vm-$i-vm"
          }

          export -f setup_vm

          echo "setting up vms..."
          ${lib.getExe pkgs.parallel-full} --verbose --tagstring {//} setup_vm {} ::: {0..20} | xargs -I {} bash -c '{} &'

          echo "sleeping"
          sleep 30
          echo "awake"

          wire_main=$(nix build --print-out-paths github:mrshmllow/wire#wire-small --no-link)
          wire_args="apply test --path ./wire -vv --ssh-accept-host -p 10"
          colmena_args="apply test --config ./colmena/hive.nix -v -p 10"

          ${lib.getExe pkgs.hyperfine} --warmup 1 --show-output --runs 1 \
            --export-markdown stats.md \
            --export-json run.json \
            "${lib.getExe self'.packages.wire-small} $wire_args" -n "wire@HEAD" \
            "$wire_main/bin/wire $wire_args" -n "wire@main" \
            "${lib.getExe' inputs.colmena_benchmarking.packages.x86_64-linux.colmena "colmena"} $colmena_args" \
                -n "colmena@pinned"
        '';
      };
    };
}
