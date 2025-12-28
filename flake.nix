{
  description = "solana-monitor";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    crane = {
      url = "github:ipetkov/crane";
    };
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, crane, rust-overlay, ... }:
    let
      linuxSystems = [
        "aarch64-linux"
        "x86_64-linux"
      ];

      supportedSystems = linuxSystems ++ [
        "aarch64-darwin"
        "x86_64-darwin"
      ];

      importPkgs = system: import nixpkgs {
        inherit system;
        overlays = [
          (import rust-overlay)
        ];
      };
    in
    flake-utils.lib.eachSystem supportedSystems (system:
      let
        pkgs = importPkgs system;
        inherit (pkgs) lib stdenv;

        rust-toolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
        craneLib = (crane.mkLib pkgs).overrideToolchain rust-toolchain;
      in
      rec {
        devShell = craneLib.devShell {
          packages = [ ];
        };

        packages = {
          solana-monitor = craneLib.buildPackage {
            src = craneLib.cleanCargoSource ./.;
            strictDeps = true;
          };
        } // lib.optionalAttrs (builtins.elem system linuxSystems) rec {
          dockerImage = pkgs.dockerTools.buildImage {
            name = "solana-monitor";
            tag = self.rev or "latest";
            copyToRoot = [
              pkgs.catatonit
              pkgs.cacert
              packages.solana-monitor
            ];
            config = {
              Labels = {
                "org.opencontainers.image.source" = "https://github.com/ZentriaMC/solana-monitor";
              } // lib.optionalAttrs (self ? rev) {
                "org.opencontainers.image.revision" = self.rev;
              };
              Env = [
                "SOLANA_MONITOR_LISTEN_ADDRESS=0.0.0.0:2112"
              ];
              Entrypoint = [ "${pkgs.catatonit}/bin/catatonit" "--" ];
              Cmd = [ "${packages.solana-monitor}/bin/solana-monitor" ];
            };
          };
        };
      });
}
