{
  lib,
  name,
  ...
}: let
  inherit (lib) types;
in {
  imports = let
    inherit (lib) mkAliasOptionModule;
  in [
    (mkAliasOptionModule ["deployment" "targetHost"] ["deployment" "target" "host"])
    (mkAliasOptionModule ["deployment" "targetUser"] ["deployment" "target" "user"])
    (mkAliasOptionModule ["deployment" "targetPort"] ["deployment" "target" "port"])
  ];

  options.deployment = {
    target = lib.mkOption {
      type = types.submodule {
        options = {
          host = lib.mkOption {
            type = types.str;
            description = "Host to connect to.";
            default = name;
          };
          hosts = lib.mkOption {
            type = types.listOf types.str;
            description = "Additional hosts to attempt to connect to, if `deployment.target.host` cannot be reached.";
            default = lib.singleton name;
            apply = list: lib.unique ([name] ++ list);
          };
          user = lib.mkOption {
            type = types.str;
            description = "User to use for ssh.";
            default = "root";
          };
          port = lib.mkOption {
            type = types.int;
            default = 22;
            description = "SSH port to use.";
          };
        };
      };
      description = "Describes the target for this node";
    };

    buildOnTarget = lib.mkOption {
      type = types.bool;
      default = true;
      description = "Whether to build the system on the target host or not.";
    };

    tags = lib.mkOption {
      type = types.listOf types.str;
      default = [];
      description = "Tags for node.";
      example = ["arm" "cloud"];
    };

    keys = lib.mkOption {
      type = types.attrsOf (types.submodule ({
        name,
        config,
        ...
      }: {
        imports = let
          inherit (lib) mkAliasOptionModule;
        in [
          (mkAliasOptionModule ["keyFile"] ["source"])
          (mkAliasOptionModule ["keyCommand"] ["source"])
          (mkAliasOptionModule ["text"] ["source"])
        ];
        options = {
          name = lib.mkOption {
            type = types.str;
            default = name;
            description = "Filename of the secret.";
          };
          destDir = lib.mkOption {
            type = types.path;
            default = "/run/keys/";
            description = "Destination directory for the secret. Change this to something other than `/run/keys/` for keys to persist past reboots.";
          };
          path = lib.mkOption {
            internal = true;
            type = types.path;
            default = "${config.destDir}/${config.name}";
          };
          group = lib.mkOption {
            type = types.str;
            default = "root";
            description = "Group to own the key.";
          };
          user = lib.mkOption {
            type = types.str;
            default = "root";
            description = "User to own the key.";
          };
          permissions = lib.mkOption {
            type = types.str;
            default = "0600";
            description = "Permissions for the key.";
          };
          source = lib.mkOption {
            type = types.oneOf [types.str types.path (types.listOf types.str)];
            description = "Source of the key. Either a path to a file, a literal string, or a command to generate the key.";
          };
        };
      }));
      description = "Secrets to be deployed to the node.";
      default = {};
      example = {
        "wireless.env" = {
          source = ["gpg" "--decrypt" "secrets/wireless.env.gpg"];
          destDir = "/etc/keys/";
        };

        "arbfile.txt" = {
          source = ./arbfile.txt;
          destDir = "/etc/arbs/";
        };

        "arberfile.txt" = {
          source = ''
            Hello World
          '';
          destDir = "/etc/arbs/";
        };
      };
    };
  };
}
