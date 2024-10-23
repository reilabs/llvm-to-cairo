//! Useful constants for use within the compiler.

/// The size of a byte on our architecture.
pub const BYTE_SIZE: usize = 8;

/// The default layout on LLVM for a 16-bit float.
///
/// The numbers are, in order: the size, the ABI alignment, and the preferred
/// alignment.
pub const DEFAULT_FLOAT_16_LAYOUT: (usize, usize, usize) = (16, 16, 16);

/// The default layout on LLVM for a 32-bit float.
///
/// The numbers are, in order: the size, the ABI alignment, and the preferred
/// alignment.
pub const DEFAULT_FLOAT_32_LAYOUT: (usize, usize, usize) = (32, 32, 32);

/// The default layout on LLVM for a 64-bit float.
///
/// The numbers are, in order: the size, the ABI alignment, and the preferred
/// alignment.
pub const DEFAULT_FLOAT_64_LAYOUT: (usize, usize, usize) = (64, 64, 64);

/// The default layout on LLVM for a 128-bit float.
///
/// The numbers are, in order: the size, the ABI alignment, and the preferred
/// alignment.
pub const DEFAULT_FLOAT_128_LAYOUT: (usize, usize, usize) = (128, 128, 128);

/// The default layout on LLVM for a 64-bit wide vector.
///
/// The numbers are, in order: the size, the ABI alignment, and the preferred
/// alignment.
pub const DEFAULT_VECTOR_64_LAYOUT: (usize, usize, usize) = (64, 64, 64);

/// The default layout on LLVM for a 128-bit wide vector.
///
/// The numbers are, in order: the size, the ABI alignment, and the preferred
/// alignment.
pub const DEFAULT_VECTOR_128_LAYOUT: (usize, usize, usize) = (128, 128, 128);

/// The default layout on LLVM for a 1-bit wide integer.
///
/// The numbers are, in order: the size, the ABI alignment, and the preferred
/// alignment.
pub const DEFAULT_INTEGER_1_LAYOUT: (usize, usize, usize) = (1, 8, 8);

/// The default layout on LLVM for an 8-bit wide integer.
///
/// The numbers are, in order: the size, the ABI alignment, and the preferred
/// alignment.
pub const DEFAULT_INTEGER_8_LAYOUT: (usize, usize, usize) = (8, 8, 8);

/// The default layout on LLVM for a 16-bit wide integer.
///
/// The numbers are, in order: the size, the ABI alignment, and the preferred
/// alignment.
pub const DEFAULT_INTEGER_16_LAYOUT: (usize, usize, usize) = (16, 16, 16);

/// The default layout on LLVM for a 32-bit wide integer.
///
/// The numbers are, in order: the size, the ABI alignment, and the preferred
/// alignment.
pub const DEFAULT_INTEGER_32_LAYOUT: (usize, usize, usize) = (32, 32, 32);

/// The default layout on LLVM for a 64-bit wide integer.
///
/// The numbers are, in order: the size, the ABI alignment, and the preferred
/// alignment.
pub const DEFAULT_INTEGER_64_LAYOUT: (usize, usize, usize) = (64, 32, 64);

/// The default layout for pointers in address space zero.
///
/// The numbers are, in order: the address space, the size, the ABI alignment,
/// the preferred alignment, and the index size.
pub const DEFAULT_POINTER_0_LAYOUT: (usize, usize, usize, usize, usize) = (0, 64, 64, 64, 64);
