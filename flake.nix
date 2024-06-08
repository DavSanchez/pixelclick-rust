{
  # Using devcontainer for now, but just in case...
  description = "@brushknight's awesome board programmed with Rust!";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    # Modules for flakes
    flake-parts.url = "github:hercules-ci/flake-parts";
    # Hooks for git (even custom)
    git-hooks.url = "github:cachix/git-hooks.nix";
    # Rust toolchains
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.rust-analyzer-src.follows = "";
    };
    # Compiling Rust projects in cacheable/composable way
    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    # For cargo audit
    advisory-db = {
      url = "github:rustsec/advisory-db";
      flake = false;
    };
  };

  outputs =
    inputs@{ flake-parts, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      imports = [ inputs.git-hooks.flakeModule ];

      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "aarch64-darwin"
      ];

      perSystem =
        {
          pkgs,
          config,
          inputs',
          self',
          ...
        }:
        let
          rustToolchain =
            with inputs'.fenix.packages;
            combine [
              complete.toolchain
              targets.riscv32imc-unknown-none-elf.latest.rust-std
            ];

          craneLib' = inputs.crane.mkLib pkgs;
          craneLib = craneLib'.overrideToolchain rustToolchain;

          src = with craneLib; cleanCargoSource (path ./.);

          # Common arguments can be set here to avoid repeating them later
          commonArgs = {
            inherit src;
            strictDeps = true;

            # buildInputs = [
            #   # Add additional build inputs here
            # ] ++ lib.optionals pkgs.stdenv.isDarwin [
            #   # Additional darwin specific inputs can be set here
            #   pkgs.libiconv
            # ];

            # Additional environment variables can be set directly
            # MY_CUSTOM_VAR = "some value";
          };

          # For coverage
          craneLibLLvmTools = craneLib'.overrideToolchain (
            rustToolchain.withComponents [
              "cargo"
              "llvm-tools"
              "rustc"
            ]
          );

          # Build *just* the cargo dependencies, so we can reuse
          # all of that work (e.g. via cachix) when running in CI
          cargoArtifacts = craneLib.buildDepsOnly commonArgs;

          # Build the actual crate itself, reusing the dependency
          # artifacts from above.
          caos-board = craneLib.buildPackage (commonArgs // { inherit cargoArtifacts; });
        in
        {
          checks = {
            # Build the crate as part of `nix flake check` for convenience
            inherit caos-board;

            # Run clippy (and deny all warnings) on the crate source,
            # again, reusing the dependency artifacts from above.
            #
            # Note that this is done as a separate derivation so that
            # we can block the CI if there are issues here, but not
            # prevent downstream consumers from building our crate by itself.
            caos-board-clippy = craneLib.cargoClippy (
              commonArgs
              // {
                inherit cargoArtifacts;
                cargoClippyExtraArgs = "--all-targets -- --deny warnings";
              }
            );

            caos-board-doc = craneLib.cargoDoc (commonArgs // { inherit cargoArtifacts; });

            # Check formatting
            caos-board-fmt = craneLib.cargoFmt { inherit src; };

            # Audit dependencies
            caos-board-audit = craneLib.cargoAudit {
              inherit src;
              inherit (inputs) advisory-db;
            };

            # Audit licenses
            caos-board-deny = craneLib.cargoDeny { inherit src; };

            # Run tests with cargo-nextest
            # Consider setting `doCheck = false` on `caos-board` if you do not want
            # the tests to run twice
            caos-board-nextest = craneLib.cargoNextest (
              commonArgs
              // {
                inherit cargoArtifacts;
                partitions = 1;
                partitionType = "count";
              }
            );
          };

          pre-commit = {
            check.enable = true;
            settings = {
              hooks = {
                actionlint.enable = true;
                cargo-check = {
                  enable = false; # ESP32 targets mess with this
                  package = rustToolchain;
                };
                clippy = {
                  enable = false; # ESP32 targets mess with this
                  packageOverrides = {
                    cargo = rustToolchain;
                    clippy = rustToolchain;
                  };
                };
                convco.enable = true;
                markdownlint.enable = false;
                nixfmt = {
                  enable = true;
                  package = pkgs.nixfmt-rfc-style;
                };
                rustfmt = {
                  enable = true;
                  packageOverrides = {
                    rustfmt = rustToolchain;
                    cargo = rustToolchain;
                  };
                };
                taplo.enable = true;
              };
            };
          };

          devShells.default = craneLib.devShell {
            # Inherit inputs from checks.
            checks = self'.checks;

            shellHook = ''
              ${config.pre-commit.installationScript}
              echo 1>&2 "Welcome to the development shell!"
            '';

            # Additional dev-shell environment variables can be set directly
            # MY_CUSTOM_DEVELOPMENT_VAR = "something else";

            # Extra inputs can be added here; cargo and rustc are provided by default.
            packages = with pkgs; [ espup ];
          };

          packages =
            {
              default = caos-board;
            }
            // pkgs.lib.optionalAttrs (!pkgs.stdenv.isDarwin) {
              caos-board-llvm-coverage = craneLibLLvmTools.cargoLlvmCov (
                commonArgs // { inherit cargoArtifacts; }
              );
            };
        };
    };
}
