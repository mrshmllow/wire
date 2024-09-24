{
  meta = {
    nixpkgs = <nixpkgs>;
  };

  node-a = {
    deployment._keys = [
      {
        name = "different-than-a";
        source = "hi";
      }
    ];
  };
}
