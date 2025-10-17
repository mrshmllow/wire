{
  lib,
  nixosOptionsDoc,
  runCommand,
  ...
}:
let
  eval = lib.evalModules {
    modules = [
      ../runtime/module/options.nix
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
in
runCommand "options-doc.md" { } ''
  cat ${optionsMd} > $out
  sed -i -e '/\*Declared by:\*/,+1d' $out
''
