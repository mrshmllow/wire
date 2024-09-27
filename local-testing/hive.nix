{
  meta = {
    nixpkgs = <nixpkgs>;
  };

  defaults = {pkgs, ...}: {
    environment.systemPackages = with pkgs; [
      vim
    ];
  };

  vm = let
    nixos-shell = builtins.fetchTarball "https://github.com/Mic92/nixos-shell/tarball/master";
  in {
    deployment = {
      target = {
        host = "127.0.0.1";
        user = "root";
        port = 2222;
      };

      buildOnTarget = false;

      tags = ["test" "arm"];
    };

    imports = [
      ./vm.nix
      (nixos-shell + "/share/modules/nixos-shell.nix")
    ];
  };
}
