{
  perSystem = {
    pre-commit = {
      settings = {
        hooks = {
          nixfmt-rfc-style.enable = true;
          rustfmt.enable = true;
          statix.enable = true;
          deadnix = {
            enable = true;
            settings.edit = true;
          };
          clippy.enable = true;
        };

      };

    };
  };

}
