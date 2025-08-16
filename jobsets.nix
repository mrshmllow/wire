# referenced from https://github.com/aciceri/nixfleet/blob/master/modules/hydra/jobsets.nix
# thank you!
{
  prs,
  ...
}:
let
  repo = {
    owner = "wires-org";
    name = "wire";
  };

  # nixpkgs
  mapAttrsToList = f: attrs: builtins.attrValues (builtins.mapAttrs f attrs);
  mapAttrs' = f: set: builtins.listToAttrs (mapAttrsToList f set);
  filterAttrs =
    pred: set:
    removeAttrs set (builtins.filter (name: !pred name set.${name}) (builtins.attrNames set));

  pull_requests = filterAttrs (_num: pr: builtins.any (label: label.name == "hydra") pr.labels) (
    builtins.fromJSON (builtins.readFile prs)
  );

  mkJobset =
    {
      enabled ? 1,
      hidden ? false,
      type ? 1,
      description ? "",
      checkinterval ? 60,
      schedulingshares ? 100,
      enableemail ? false,
      emailoverride ? "",
      keepnr ? 3,
      flake,
    }:
    {
      inherit
        enabled
        hidden
        type
        description
        checkinterval
        schedulingshares
        enableemail
        emailoverride
        keepnr
        flake
        ;
    };

  mkSpec =
    contents:
    let
      escape = builtins.replaceStrings [ ''"'' ] [ ''\"'' ];
      contentsJson = builtins.toJSON contents;
    in
    builtins.derivation {
      name = "spec.json";
      system = "x86_64-linux";
      preferLocalBuild = true;
      allowSubstitutes = false;
      builder = "/bin/sh";
      args = [
        (builtins.toFile "builder.sh" ''
          echo "${escape contentsJson}" > $out
        '')
      ];
    };
in
{
  jobsets = mkSpec (
    {
      main = mkJobset {
        description = "${repo.name}'s main branch";
        flake = "git+ssh://git@github.com/${repo.owner}/${repo.name}?ref=main";
      };
    }
    // (mapAttrs' (n: pr: {
      name = "pr_${n}";
      value = mkJobset {
        description = pr.title;
        flake = "git+ssh://git@github.com/${repo.owner}/${repo.name}?ref=${pr.head.ref}";
      };
    }) pull_requests)
  );
}
