let
  sources = import ./npins;
  wire = import sources.wire;
in
  wire.makeHive {
    # give wire nixpkgs from npins
    meta.nixpkgs = import sources.nixpkgs {};

    # Continue to next How-To guide to fill this section
  }
