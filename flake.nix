{
  description = "solana-monitor";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.flake-utils.follows = "flake-utils";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, crane, rust-overlay, ... }:
    let
      supportedSystems = [
        "aarch64-darwin"
        "aarch64-linux"
        "x86_64-darwin"
        "x86_64-linux"
      ];
    in
    flake-utils.lib.eachSystem supportedSystems (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [
            (import rust-overlay)
          ];
        };
        inherit (pkgs) lib stdenv;

        rust-toolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
        craneLib = (crane.mkLib pkgs).overrideToolchain rust-toolchain;
      in
      rec {
        devShell = craneLib.devShell {
          packages = [ ];
        };

        packages.solana-monitor = craneLib.buildPackage rec {
          src = craneLib.cleanCargoSource ./.;
          strictDeps = true;

          nativeBuildInputs = lib.optionals stdenv.isLinux [
            pkgs.pkg-config
          ];

          buildInputs = lib.optionals stdenv.isLinux [
            pkgs.openssl
          ] ++ lib.optionals stdenv.isDarwin [
            pkgs.darwin.apple_sdk.frameworks.CoreFoundation
            pkgs.darwin.apple_sdk.frameworks.Security
            pkgs.libiconv
          ];
        };

        packages.dockerImage = pkgs.dockerTools.buildImage {
          name = "solana-monitor";
          tag = self.rev or "latest";
          copyToRoot = [
            pkgs.catatonit
            pkgs.cacert
            packages.solana-monitor
          ];
          config = {
            Labels = {
              "org.opencontainers.image.source" = "https://github.com/callStatic/solana-monitor";
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
      });
}
