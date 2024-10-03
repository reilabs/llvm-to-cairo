# `FlatLowered` Intermediate Representation

The `FlatLowered` Intermediate Representation (`FLIR`) is an intermediate representation for the
LLVM to Cairo project that is based on Cairo's `FlatLowered` but tailored for our use-case.

In particular, it removes any dependency on the [Salsa](https://github.com/salsa-rs/salsa) database
structures, as well as:

- Allowing round-tripping to and from `FlatLowered`.
- Forming the basis of the `.flo` (FlatLowered Object) object format for exchange between tools in
  the LTC pipeline.
- Adding support for features (such as linkage and relocations) that are not supported by the
  `FlatLowered`.

While we could spend the time to add these features to upstream `FlatLowered`, having our own IR
means that we can iterate and experiment far more rapidly as needed. Some portions of this work
_may_ be upstreamed in the future, but we make no guarantees.
