# This file defines the actual package.
#
# We want to be able to run commands from a nix shell using this package
# definition.
{ craneLib, lib, llvmPackages_18, libiconv, stdenv }: let
  src = craneLib.cleanCargoSource ./.;
in craneLib.buildPackage {
  inherit src;

  # Disallow confusing buildInputs and nativeBuildInputs for sanity.
  strictDeps = true;

  # Things that are needed at build time on the system doing building.
  nativeBuildInputs = [
    llvmPackages_18.llvm
  ];

  # The things that we need available at build and runtime on the target system.
  buildInputs = [
    llvmPackages_18.llvm
  ] ++ lib.optionals stdenv.hostPlatform.isDarwin [
    libiconv
  ];

  # We name this so we can quickly `nix-run`
  meta.mainProgram = "llvm-to-cairo";
}
