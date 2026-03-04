{
  description = "Zellij cmd-k command palette plugin";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      rust-overlay,
      flake-utils,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };
        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          targets = [ "wasm32-wasip1" ];
        };
        wasmTarget = "wasm32-wasip1";

        rustPlatform = pkgs.makeRustPlatform {
          cargo = rustToolchain;
          rustc = rustToolchain;
        };

        zellij-cmd-k = pkgs.stdenv.mkDerivation {
          pname = "zellij-cmd-k";
          version = "0.1.0";
          src = ./.;

          cargoDeps = rustPlatform.fetchCargoVendor {
            src = ./.;
            hash = "sha256-HI1XrI6vQ0uRlhFalB0f0Z360E2RiZunaRp2TiYH64k=";
          };

          nativeBuildInputs = [
            rustToolchain
            rustPlatform.cargoSetupHook
            pkgs.pkg-config
          ];

          buildInputs =
            [ pkgs.openssl ]
            ++ pkgs.lib.optionals pkgs.stdenv.hostPlatform.isDarwin [
              pkgs.apple-sdk_15
            ];

          buildPhase = ''
            cargo build --target ${wasmTarget} --release
          '';

          installPhase = ''
            mkdir -p $out
            cp target/${wasmTarget}/release/zellij-cmd-k.wasm $out/
          '';
        };
      in
      {
        packages = {
          default = zellij-cmd-k;
          inherit zellij-cmd-k;
        };

        devShells.default = pkgs.mkShell {
          buildInputs = [
            rustToolchain
            pkgs.zellij
          ];
        };

        formatter = pkgs.nixfmt-rfc-style;
      }
    );
}
