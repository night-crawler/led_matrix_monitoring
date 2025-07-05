{
  description = "LED Matrix Monitoring";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    crane.url = "github:ipetkov/crane";
    flake-utils.url = "github:numtide/flake-utils";
    advisory-db = {
      url = "github:rustsec/advisory-db";
      flake = false;
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      crane,
      flake-utils,
      advisory-db,
      ...
    }:
    let
      # NixOS module for the led-matrix-monitoring service
      nixosModule =
        {
          config,
          lib,
          pkgs,
          ...
        }:
        let
          format = pkgs.formats.toml { };
          cfg = config.services.led-matrix-monitoring;
          defaultConfig = builtins.fromTOML (builtins.readFile ./example_config.toml);

          # Check if led-matrix-daemon socket is configured
          daemonSocketEnabled = config.systemd.sockets.led-matrix-daemon.enable or false;
          daemonSocketPath = config.systemd.sockets.led-matrix-daemon.socketConfig.ListenStream or null;

          # Set socket path from daemon if available
          socketSettings = lib.mkIf daemonSocketEnabled {
            socket = daemonSocketPath;
          };

          # Generate the final TOML configuration
          finalConfig = lib.recursiveUpdate (lib.recursiveUpdate defaultConfig socketSettings) cfg.settings;

          configFile = format.generate "led-matrix-monitoring.toml" finalConfig;
        in
        {
          options.services.led-matrix-monitoring = {
            enable = lib.mkEnableOption "LED Matrix Monitoring Service";

            package = lib.mkOption {
              type = lib.types.package;
              default = self.packages.${pkgs.system}.default;
              description = "The led-matrix-monitoring package to use.";
            };

            settings = lib.mkOption {
              type = lib.types.submodule {
                freeformType = lib.types.attrs;
              };
              default = { };
              description = "Configuration for led-matrix-monitoring.";
              example = lib.literalExpression ''
                {
                  socket = "/run/led-matrix/led-matrix.sock";
                  collector.max_history_samples = 20;
                  collector.sample_interval = "200ms";
                }
              '';
            };
          };

          config = lib.mkIf cfg.enable {
            # Create /etc/led_matrix directory and set max_brightness_value file
            environment.etc."led_matrix/max_brightness_value" = {
              mode = "0644";
              text = "255";
            };

            systemd.services.led-matrix-monitoring = {
              description = "LED Matrix Monitoring Service";
              after = [
                "network.target"
                "led-matrix-daemon.socket"
                "led-matrix-daemon.service"
              ];
              requires = [
                "led-matrix-daemon.socket"
                "led-matrix-daemon.service"
              ];
              wantedBy = [ "multi-user.target" ];

              # unitConfig = {
              #   JoinsNamespaceOf = [
              #     "led-matrix-daemon.service"
              #     "led-matrix-daemon.socket"
              #   ];
              #   RequiresMountsFor = "/run/led-matrix";
              # };

              serviceConfig = {
                Type = "simple";
                ExecStart = "${cfg.package}/bin/led_matrix_monitoring --config=${configFile}";
                Restart = "on-failure";

                # PrivateMounts = false;
                # ProtectSystem = "off";
                # ProtectHome = false;

                User = "root";
                Group = "root";
              };
            };
          };
        };
    in
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = nixpkgs.legacyPackages.${system};

        inherit (pkgs) lib;

        craneLib = crane.mkLib pkgs;
        src = craneLib.cleanCargoSource ./.;

        # Common arguments can be set here to avoid repeating them later
        commonArgs = {
          inherit src;
          strictDeps = true;

          buildInputs =
            [
              # Add additional build inputs here
            ]
            ++ lib.optionals pkgs.stdenv.isDarwin [
              # Additional darwin specific inputs can be set here
              pkgs.libiconv
            ];

          # Additional environment variables can be set directly
          # MY_CUSTOM_VAR = "some value";
        };

        # Build *just* the cargo dependencies, so we can reuse
        # all of that work (e.g. via cachix) when running in CI
        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        # Build the actual crate itself, reusing the dependency
        # artifacts from above.
        led-matrix-monitoring = craneLib.buildPackage (
          commonArgs
          // {
            inherit cargoArtifacts;
          }
        );
      in
      {
        checks = {
          # Build the crate as part of `nix flake check` for convenience
          inherit led-matrix-monitoring;

          # Run clippy (and deny all warnings) on the crate source,
          # again, reusing the dependency artifacts from above.
          #
          # Note that this is done as a separate derivation so that
          # we can block the CI if there are issues here, but not
          # prevent downstream consumers from building our crate by itself.
          led-matrix-monitoring-clippy = craneLib.cargoClippy (
            commonArgs
            // {
              inherit cargoArtifacts;
              cargoClippyExtraArgs = "--all-targets -- --deny warnings";
            }
          );

          led-matrix-monitoring-doc = craneLib.cargoDoc (
            commonArgs
            // {
              inherit cargoArtifacts;
            }
          );

          # Check formatting
          led-matrix-monitoring-fmt = craneLib.cargoFmt {
            inherit src;
          };

          led-matrix-monitoring-toml-fmt = craneLib.taploFmt {
            src = pkgs.lib.sources.sourceFilesBySuffices src [ ".toml" ];
            # taplo arguments can be further customized below as needed
            # taploExtraArgs = "--config ./taplo.toml";
          };

          # Audit dependencies
          led-matrix-monitoring-audit = craneLib.cargoAudit {
            inherit src advisory-db;
          };

          # Audit licenses
          led-matrix-monitoring-deny = craneLib.cargoDeny {
            inherit src;
          };

          # Run tests with cargo-nextest
          # Consider setting `doCheck = false` on `led-matrix-monitoring` if you do not want
          # the tests to run twice
          led-matrix-monitoring-nextest = craneLib.cargoNextest (
            commonArgs
            // {
              inherit cargoArtifacts;
              partitions = 1;
              partitionType = "count";
              cargoNextestPartitionsExtraArgs = "--no-tests=pass";
            }
          );
        };

        packages = {
          default = led-matrix-monitoring;
        };

        apps.default = flake-utils.lib.mkApp {
          drv = led-matrix-monitoring;
        };

        devShells.default = craneLib.devShell {
          # Inherit inputs from checks.
          checks = self.checks.${system};

          # Additional dev-shell environment variables can be set directly
          # MY_CUSTOM_DEVELOPMENT_VAR = "something else";

          # Extra inputs can be added here; cargo and rustc are provided by default.
          packages = [
            # pkgs.ripgrep
          ];
        };
      }
    )
    // {
      # Export the NixOS module
      nixosModules.default = nixosModule;
    };
}
