let
  sources = import ./npins;
  wire = import sources.wire;
in
  wire.makeHive {
    # give wire nixpkgs from npins
    meta.nixpkgs = import sources.nixpkgs {};

    # ...
  }
