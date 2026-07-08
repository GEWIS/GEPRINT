{
  description = "geprint — Dioxus fullstack CUPS print server";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  # Consumers of this flake automatically pull prebuilt artifacts from the
  # GEWIS Cachix cache.
  nixConfig = {
    extra-substituters = [ "https://gewis.cachix.org" ];
    extra-trusted-public-keys = [ "gewis.cachix.org-1:bOcor+MaaLuUJN0Yj/IHCXsOQWm/RxSokm6BHGcbF5k=" ];
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils }:
    let
      # NixOS module is system-independent.
      nixosModules.default = import ./nix/module.nix self;
    in
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) (import ./nix/wasm-bindgen.nix) ];
        pkgs = import nixpkgs { inherit system overlays; };
        rustToolchain = pkgs.rust-bin.stable."1.95.0".default.override {
          targets = [ "wasm32-unknown-unknown" ];
        };
      in
      {
        packages.default = pkgs.callPackage ./nix/package.nix {
          inherit rustToolchain;
        };

        devShells.default = pkgs.mkShell {
          packages = [
            rustToolchain
            pkgs.dioxus-cli
            pkgs.cups         # provides lp / lpstat for local testing
            pkgs.wasm-bindgen-cli
            pkgs.pkg-config
            pkgs.openssl
          ];
          # dx/wasm builds sometimes need this.
          RUST_SRC_PATH = "${rustToolchain}/lib/rustlib/src/rust/library";
        };
      })
    // {
      inherit nixosModules;
    };
}
