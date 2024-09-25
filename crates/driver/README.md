# LLVM to Cairo Compiler Driver

The compiler driver is responsible for marrying up the compilation process that we control with the
further parts of the Cairo compilation pipeline. This includes generating and checking Sierra, and
the final emission of CASM.
