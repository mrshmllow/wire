{
  lib,
  rustPlatform,
}:
rustPlatform.buildRustPackage {
  pname = "wire";
  version = "0.1.0";
  cargoLock.lockFile = ./Cargo.lock;
  src = lib.cleanSource ./.;
}
