{
  pkgs,
  lib,
  nixosOptionsDoc,
  runCommand,
  wire,
  nodejs,
  pnpm,
  stdenv,
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

  pkg = builtins.fromJSON (builtins.readFile ./package.json);
in
stdenv.mkDerivation (finalAttrs: {
  inherit (pkg) version;
  pname = pkg.name;
  nativeBuildInputs = [
    wire
    nodejs
    pnpm.configHook
  ];
  src = ./.;
  pnpmDeps = pnpm.fetchDeps {
    inherit (finalAttrs) pname version src;
    hash = "sha256-7ThshGf9tkCwSaz4DMTXxmjhN+2g0dErgzpgzJ2gv8Y";
  };
  patchPhase = ''
    cat ${optionsDoc} >> ./reference/module.md
    wire inspect --markdown-help > ./reference/cli.md
  '';
  buildPhase = "pnpm run build > build.log 2>&1";
  installPhase = "cp .vitepress/dist -r $out";
  DEBUG = "*";
})
