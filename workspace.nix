# This file defines the actual package.
#
# We want to be able to run commands from a nix shell using this package
# definition.
{
  craneLib,
  lib,
  llvmPackages_18,
  libiconv,
  stdenv,
}:
 let
  workspaceToml = lib.importTOML ./Cargo.toml;
  # nb: if any crates set `version.workspace = false`, this will need to be updated,
  # but otherwise we can use the workspace version for every crate.
  version = workspaceToml.workspace.package.version;
  workspaceMemberPaths = workspaceToml.workspace.members;

  # Attrsets in this will add additional arguments to craneLib.buildPackage for the
  # crate with the matching package name.
  crateSpecificArgs = {
    ltc-cli = {
      meta.mainProgram = "ltc";
    };
  };

  # These are added as arguments to both the Cargo dependencies and each crate in the
  # workspace.
  commonArgs = {
    src = craneLib.cleanCargoSource ./.;

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
  };

  # The Cargo.lock dependencies are global to the workspace, so we can build them
  # separately, and thus only build them once for the whole workspace.
  workspaceDeps = craneLib.buildDepsOnly (commonArgs // {
    # Workspaces don't have names, so we'll give it the repo name for the dependencies.
    pname = "llvm-to-cairo-deps";
    inherit version;
  });

  # A list of all the crates in this workspace, where each item in the list is a
  # name-value pair for `lib.listToAttrs`. That way we get the crate names as the
  # attrset keys.
  memberCrates = lib.forEach workspaceMemberPaths (cratePath:
    let
      # The syntax for dynamic relative paths is weird, I know.
      crateToml = lib.importTOML (./. + "/${cratePath}/Cargo.toml");
      pname = crateToml.package.name;
      # Add any arguments specific to this crate's args, if there are any.
      crateOverrideArgs = crateSpecificArgs.${pname} or { };
    in {
      name = pname;
      value = craneLib.buildPackage (commonArgs // {
        inherit pname version;

        # Note that `-p` takes the Cargo package name, not the workspace member path.
        cargoExtraArgs = "-p ${pname}";
        cargoArtifacts = workspaceDeps;

      } // crateOverrideArgs);
    }
  );

in
lib.listToAttrs memberCrates
