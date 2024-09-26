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
        rustVersion = (builtins.fromTOML (builtins.readFile ./Cargo.toml)).workspace.package.rust-version;

        # Then we set up our libraries for building this thing.
        pkgs = nixpkgs.legacyPackages.${system};
        inherit (pkgs) lib;
        fenixLib = fenix.packages.${system};
        fenixStable = fenixLib.fromToolchainName {
            name = rustVersion;
            sha256 = "sha256-VZZnlyP69+Y3crrLHQyJirqlHrTtGTsyiSnZB8jEvVo=";
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

        # Collect our workspace packages, including our application.
        workspacePackages = pkgs.callPackage ./workspace.nix {
          inherit craneLib;
        };

        # Filter out things that aren't derivations for the `packages` output, or Nix gets mad.
        llvmToCairo = lib.filterAttrs (lib.const lib.isDerivation) workspacePackages;

        # And for convenience, collect all the workspace members into a single derivation,
        # so we can check they all compile with one command, `nix build '.#all'`.
        all = pkgs.symlinkJoin {
          name = "llvm-to-cairo-all";
          paths = lib.attrValues llvmToCairo;
        };

        # We get your default shell to make sure things feel familiar in the dev shell.
        getUserShellCommand = if pkgs.stdenv.hostPlatform.isDarwin then
          "dscl . -read ~ UserShell | cut -d ' ' -f2"
        else
          "getent passwd $USER | cut -d ':' -f7";
      in {
        packages = {
          inherit all;
          default = llvmToCairo.ltc-cli;
        } // llvmToCairo;

        # The default dev shell puts you in your native shell to make things feel happy.
        devShells.default = craneLib.devShell {
          LLVM_SYS_180_PREFIX = "${pkgs.lib.getDev pkgs.llvmPackages_18.libllvm}";
          inputsFrom = lib.attrValues llvmToCairo;

          packages = [
            pkgs.nodejs_22
          ];

          shellHook = ''
          exec $(${getUserShellCommand})
          '';
        };

        # The dev shell for CI allows it to interpret commands properly.
        devShells.ci = craneLib.devShell {
          LLVM_SYS_180_PREFIX = "${pkgs.lib.getDev pkgs.llvmPackages_18.libllvm}";
          inputsFrom = lib.attrValues llvmToCairo;
        };
      }
    );
}
