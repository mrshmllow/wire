let
  lock = builtins.fromJSON (builtins.readFile ./flake.lock);
  nodeName = lock.nodes.root.inputs.flake-compat;
  tarball = fetchTarball {
    url = lock.nodes.${nodeName}.locked.url or "https://github.com/edolstra/flake-compat/archive/${lock.nodes.${nodeName}.locked.rev}.tar.gz";
    sha256 = lock.nodes.${nodeName}.locked.narHash;
  };
in {
  # we split these up so we can preload the tarball within testing vms
  inherit tarball;
  flake = (import tarball {src = ./.;}).defaultNix;
}
