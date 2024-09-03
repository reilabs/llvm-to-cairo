# A flake that sets up the necessary development environment for things.
{
  description = "LLVM to CairoVM";

  # The things that we want to pin to.
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    crane.url = "github:ipetkov/crane";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  # The results of our flake.
  outputs = { self, nixpkgs, flake-utils, crane, fenix }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        # We grab our expected rust version from the Cargo.toml.
        rustVersion = (builtins.fromTOML (builtins.readFile ./Cargo.toml)).package.rust-version;

        # Then we set up our libraries for building this thing.
        pkgs = nixpkgs.legacyPackages.${system};
        fenixLib = fenix.packages.${system};
        fenixStable = fenixLib.fromToolchainName {
            name = rustVersion;
            sha256 = "sha256-3jVIIf5XPnUU1CRaTyAiO0XHVbJl12MSx3eucTXCjtE=";
        };

        # As we want nightly Rustfmt, we have to build a custom toolchain.
        fenixToolchain = fenixLib.combine [
          fenixLib.latest.rustfmt  # `fenixLib.latest` is specifically the latest nightly
          (fenixStable.withComponents [
            "cargo"
            "clippy"
            "rust-docs"
            "rust-src"
            "rust-std"
            "rustc"
          ])
        ];

        # The crane library configures the Rust toolchain, along with the components we expect it
        # to have.
        craneLib = (crane.mkLib pkgs).overrideToolchain fenixToolchain;

        # Then we build our actual package, which is our application.
        llvmToCairo = pkgs.callPackage ./package.nix {
          inherit craneLib;
        };

        # We get your default shell to make sure things feel familiar in the dev shell.
        getUserShellCommand = if pkgs.stdenv.hostPlatform.isDarwin then
          "dscl . -read ~ UserShell | cut -d ' ' -f2"
        else
          "getent passwd $USER | cut -d ':' -f7";
      in {
        packages = {
          inherit llvmToCairo;
          default = llvmToCairo;
        };

        # The default dev shell puts you in your native shell to make things feel happy.
        devShells.default = craneLib.devShell {
          LLVM_SYS_180_PREFIX = "${pkgs.lib.getDev pkgs.llvmPackages_18.libllvm}";
          inputsFrom = [
            llvmToCairo
          ];

          shellHook = ''
          exec $(${getUserShellCommand})
          '';
        };

        # The dev shell for CI allows it to interpret commands properly.
        devShells.ci = craneLib.devShell {
          LLVM_SYS_180_PREFIX = "${pkgs.lib.getDev pkgs.llvmPackages_18.libllvm}";
          inputsFrom = [
            llvmToCairo
          ];
        };
      }
    );
}
