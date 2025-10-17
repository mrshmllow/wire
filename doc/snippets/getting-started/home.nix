{system, ...}: let
  wire = import (
    # [!code ++]
    builtins.fetchTarball "https://github.com/mrshmllow/wire/archive/refs/heads/main.tar.gz" # [!code ++]
  ); # [!code ++]
in {
  home.packages = [
    wire.packages.${system}.wire # [!code ++]
  ];

  # ...
}
