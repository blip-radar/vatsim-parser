{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = inputs:
    inputs.flake-parts.lib.mkFlake {inherit inputs;} {
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "riscv64-linux"
      ];

      perSystem = {
        pkgs,
        system,
        ...
      }: {
        _module.args.pkgs = import inputs.nixpkgs {
          inherit system;
          overlays = [inputs.rust-overlay.overlays.default];
        };

        devShells = {
          default = pkgs.mkShell.override {stdenv = pkgs.clangMultiStdenv;} {
            nativeBuildInputs = with pkgs; [pkg-config];
            buildInputs = with pkgs; [
              rust-bin.stable.latest.default
              cargo-tarpaulin cargo-watch
            ];

          };
        };

        formatter = pkgs.nixpkgs-fmt;
      };
    };
}
