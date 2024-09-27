{
  description = "Extended FS flake";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    { self
    , nixpkgs
    , flake-utils
    , rust-overlay
    , crane
    }:
    flake-utils.lib.eachDefaultSystem (system:
    let
      overlays = [ rust-overlay.overlays.default ];
      pkgs = import nixpkgs { inherit system overlays; };

      rust = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;

      craneLib = (crane.mkLib nixpkgs.legacyPackages.${system}).overrideToolchain rust;

      commonArgs = {
        src = craneLib.cleanCargoSource self;
      };

      cargoArtifacts = craneLib.buildDepsOnly commonArgs;
    in
    rec {
      devShells.default = craneLib.devShell {
        packages = with pkgs; [
          cargo-deny
          git
        ];

        RUSTDOCFLAGS = "--cfg docsrs";
      };

      packages.efs = craneLib.buildPackage (commonArgs // {
        inherit cargoArtifacts;
      });

      checks.efs = packages.efs;

      checks.efs-clippy = craneLib.cargoClippy (commonArgs // {
        inherit cargoArtifacts;
      });
    });
}
