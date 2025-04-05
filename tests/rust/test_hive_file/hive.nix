{
  meta = {
    nixpkgs = <nixpkgs>;
  };

  node-a = {
    deployment = {
      target = {
        host = "192.168.122.96";
        user = "root";
      };
    };

    nixpkgs.hostPlatform = "x86_64-linux";
  };
}
