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
          ...
        }:
        let
          rustToolchainManifest = pkgs.lib.recursiveUpdate (builtins.fromJSON (builtins.readFile "${inputs.fenix}/data/stable.json")) (
            builtins.fromJSON (builtins.readFile ./esp-rs.json)
          );
          rustToolchain = (inputs'.fenix.packages.fromManifest rustToolchainManifest).toolchain;
        in
        {

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

          devShells.default = pkgs.mkShell {
            name = "esp32s3";

            # This doesn't exactly make the builds isolated and Nix-like, but it's quick enough for now
            shellHook = ''
              ${config.pre-commit.installationScript}

              echo 1>&2 "Welcome to the ESP32S3 with Rust development shell!"

              # echo 1>&2 "Setting up Rust toolchain for ESP32..."
              # ${pkgs.espup}/bin/espup install --targets esp32s3 --log-level debug --export-file ./export-esp.sh

              # source ./export-esp.sh
            '';

            packages =
              [
                rustToolchain # Needed?
              ]
              ++ (with pkgs; [
                # espup # For updating manually?
                # To load binary into the board
                # run with `espflash flash --monitor`
                espflash
                probe-rs
              ]);
            # inputsFrom = [];
          };
        };
    };
}
