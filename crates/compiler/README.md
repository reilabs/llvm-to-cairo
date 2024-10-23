# Hieratika Compiler

This crate implements the full compilation behavior from LLVM IR to our `.flo` object format, but no
further parts of the process. This is combined with the downstream linking and emission steps by the
[compiler driver](../driver).
