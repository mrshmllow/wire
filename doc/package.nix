{
  pkgs,
  lib,
  nixosOptionsDoc,
  runCommand,
  wire,
  ...
}:
let
  eval = lib.evalModules {
    modules = [
      ../runtime/module.nix
      {
        options._module.args = lib.mkOption {
          internal = true;
        };
      }
    ];
    specialArgs = {
      name = "‹node name›";
      nodes = { };
    };
  };

  optionsMd =
    (nixosOptionsDoc {
      inherit (eval) options;
    }).optionsCommonMark;

  optionsDoc = runCommand "options-doc.md" { } ''
    cat ${optionsMd} > $out
    sed -i -e '/\*Declared by:\*/,+1d' $out
  '';
in
pkgs.stdenv.mkDerivation {
  name = "wire-docs";
  buildInputs = with pkgs; [
    mdbook
    mdbook-alerts
  ];
  src = ./.;
  buildPhase = ''
    cat ${optionsDoc} >> ./src/modules/README.md
    ${lib.getExe wire} inspect --markdown-help > ./src/cli/README.md
    mdbook build -d $out
  '';
}
