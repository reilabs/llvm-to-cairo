//! This module contains the definition of the [`DataLayout`] struct, as well as
//! utilities for querying and reasoning about said layouts.

use chumsky::{
    error::Simple,
    prelude::{choice, just},
    Parser,
};
use hieratika_errors::compile::{Error, Result};

use crate::constant::{
    BYTE_SIZE,
    DEFAULT_FLOAT_128_LAYOUT,
    DEFAULT_FLOAT_16_LAYOUT,
    DEFAULT_FLOAT_32_LAYOUT,
    DEFAULT_FLOAT_64_LAYOUT,
    DEFAULT_INTEGER_16_LAYOUT,
    DEFAULT_INTEGER_1_LAYOUT,
    DEFAULT_INTEGER_32_LAYOUT,
    DEFAULT_INTEGER_64_LAYOUT,
    DEFAULT_INTEGER_8_LAYOUT,
    DEFAULT_POINTER_0_LAYOUT,
    DEFAULT_VECTOR_128_LAYOUT,
    DEFAULT_VECTOR_64_LAYOUT,
};

/// Information about the expected data-layout for this module.
///
/// # Defaulting
///
/// LLVM starts with a default specification of the data-layout that is possibly
/// overridden by the data-layout string. This struct implements this behavior,
/// so if you want a defaulted layout, either call [`DataLayout::new`] with an
/// empty string, or use the [`Default`] instance.
#[derive(Clone, Debug, PartialEq)]
pub struct DataLayout {
    /// The endianness used in this data layout.
    pub endianness: Endianness,

    /// The mangling scheme used by this data layout.
    pub mangling: Mangling,

    /// The natural alignment of the stack in bits.
    pub stack_alignment: usize,

    /// The index of the address space that corresponds to program memory.
    pub program_address_space: usize,

    /// The index of the address space that corresponds to globals.
    pub global_address_space: usize,

    /// The index of the address space for allocations.
    pub alloc_address_space: usize,

    /// The layout of pointers.
    pub pointer_layouts: Vec<PointerLayout>,

    /// The layout of the various integer types.
    pub integer_layouts: Vec<IntegerLayout>,

    /// The layout of the various vector types.
    pub vector_layouts: Vec<VectorLayout>,

    /// The layout of the various floating-point types.
    pub float_layouts: Vec<FloatLayout>,

    /// The layout of aggregate types.
    pub aggregate_layout: AggregateLayout,

    /// The layout of function pointers.
    pub function_pointer_layout: FunctionPointerLayout,

    /// The integer widths natively supported by the CPU in this layout.
    pub native_integer_widths: NativeIntegerWidths,

    /// The address space numbers in which pointers should be treated as
    /// non-integral.
    pub nointptr_address_spaces: NonIntegralPointerAddressSpaces,
}

impl DataLayout {
    /// Constructs a new data layout description from the provided
    /// `layout_string`.
    ///
    /// If any portion of the data layout specification is left unspecified,
    /// then the default data layout specification is used in its place as
    /// described [here](https://llvm.org/docs/LangRef.html#data-layout). In
    /// addition, we:
    ///
    /// - Default to 32 and 64-bit for the native integer widths.
    /// - Default to independent function pointers aligned to 64 bits.
    /// - Default to the ELF mangling scheme if none is specified.
    ///
    /// # Errors
    ///
    /// - [`Error::InvalidDataLayoutSpecification`] if the provided
    ///   `layout_string` cannot be parsed as a data layout specification.
    pub fn new(layout_string: &str) -> Result<Self> {
        let parts = layout_string.split('-');

        // Allocate a default that is KNOWINGLY INCOMPLETE. This is not a valid layout
        // to return, but serves as a place to stick our specifications as we parse
        // them.
        let mut layout = DataLayout {
            endianness:              Endianness::Little,
            mangling:                Mangling::ELF,
            stack_alignment:         0,
            program_address_space:   0,
            global_address_space:    0,
            alloc_address_space:     0,
            pointer_layouts:         vec![],
            integer_layouts:         vec![],
            vector_layouts:          vec![],
            float_layouts:           vec![],
            aggregate_layout:        AggregateLayout {
                abi_alignment:       0,
                preferred_alignment: 64,
            },
            function_pointer_layout: FunctionPointerLayout {
                ptr_type:      FunctionPointerType::Independent,
                abi_alignment: 64,
            },
            native_integer_widths:   NativeIntegerWidths {
                widths: vec![32, 64],
            },
            nointptr_address_spaces: NonIntegralPointerAddressSpaces {
                address_spaces: Vec::new(),
            },
        };

        // Parse out each specification from the data-layout string.
        for part in parts {
            if let Ok(e) = Endianness::parser().parse(part) {
                layout.endianness = e;
            } else if let Ok(m) = Mangling::parser().parse(part) {
                layout.mangling = m;
            } else if let Ok(align) = parsing::stack_alignment().parse(part) {
                layout.stack_alignment = align;
            } else if let Ok(p_addr) = parsing::program_address_space().parse(part) {
                layout.program_address_space = p_addr;
            } else if let Ok(g_addr) = parsing::global_address_space().parse(part) {
                layout.global_address_space = g_addr;
            } else if let Ok(a_addr) = parsing::alloc_address_space().parse(part) {
                layout.alloc_address_space = a_addr;
            } else if let Ok(ptr_spec) = PointerLayout::parser().parse(part) {
                layout.pointer_layouts.push(ptr_spec);
            } else if let Ok(int_spec) = IntegerLayout::parser().parse(part) {
                layout.integer_layouts.push(int_spec);
            } else if let Ok(vec) = VectorLayout::parser().parse(part) {
                layout.vector_layouts.push(vec);
            } else if let Ok(float_spec) = FloatLayout::parser().parse(part) {
                layout.float_layouts.push(float_spec);
            } else if let Ok(agg) = AggregateLayout::parser().parse(part) {
                layout.aggregate_layout = agg;
            } else if let Ok(f_ptr) = FunctionPointerLayout::parser().parse(part) {
                layout.function_pointer_layout = f_ptr;
            } else if let Ok(iw) = NativeIntegerWidths::parser().parse(part) {
                layout.native_integer_widths = iw;
            } else if let Ok(npa) = NonIntegralPointerAddressSpaces::parser().parse(part) {
                layout.nointptr_address_spaces = npa;
            } else if part.is_empty() {
                // We don't know if empty parts are allowed, so we just behave permissively
                // here. It cannot introduce any bugs to be permissive in this case.
                continue;
            } else {
                Err(Error::InvalidDataLayoutSpecification(
                    layout_string.to_string(),
                    part.to_string(),
                ))?;
            }
        }

        // Finally we add the defaults for vector-typed fields as these have to be done
        // after parsing.
        layout.pointer_layouts = Self::pointer_specs_or_defaults(layout.pointer_layouts);
        layout.integer_layouts = Self::int_specs_or_defaults(layout.integer_layouts);
        layout.vector_layouts = Self::vec_specs_or_defaults(layout.vector_layouts);
        layout.float_layouts = Self::float_specs_or_defaults(layout.float_layouts);

        // Finally we can build the data layout
        Ok(layout)
    }

    /// Augments the parsed floating-point layout specifications with any
    /// missing information based on the defaults for LLVM's data layout.
    fn float_specs_or_defaults(mut specs: Vec<FloatLayout>) -> Vec<FloatLayout> {
        let float_defaults = [
            DEFAULT_FLOAT_16_LAYOUT,
            DEFAULT_FLOAT_32_LAYOUT,
            DEFAULT_FLOAT_64_LAYOUT,
            DEFAULT_FLOAT_128_LAYOUT,
        ];

        for (size, abi_alignment, preferred_alignment) in float_defaults {
            if !specs.iter().any(|f| f.size == size) {
                specs.push(FloatLayout {
                    size,
                    abi_alignment,
                    preferred_alignment,
                });
            }
        }

        specs.sort();
        specs
    }

    /// Augments the parsed vector layout specifications with any missing
    /// information based on the defaults for LLVM's data layout.
    fn vec_specs_or_defaults(mut specs: Vec<VectorLayout>) -> Vec<VectorLayout> {
        let vector_layouts = [DEFAULT_VECTOR_64_LAYOUT, DEFAULT_VECTOR_128_LAYOUT];

        for (size, abi_alignment, preferred_alignment) in vector_layouts {
            if !specs.iter().any(|v| v.size == size) {
                specs.push(VectorLayout {
                    size,
                    abi_alignment,
                    preferred_alignment,
                });
            }
        }

        specs.sort();
        specs
    }

    /// Augments the parsed integer specifications with any missing information
    /// based on the defaults for LLVM's data layout.
    fn int_specs_or_defaults(mut specs: Vec<IntegerLayout>) -> Vec<IntegerLayout> {
        let integer_layouts = [
            DEFAULT_INTEGER_1_LAYOUT,
            DEFAULT_INTEGER_8_LAYOUT,
            DEFAULT_INTEGER_16_LAYOUT,
            DEFAULT_INTEGER_32_LAYOUT,
            DEFAULT_INTEGER_64_LAYOUT,
        ];

        for (size, abi_alignment, preferred_alignment) in integer_layouts {
            if !specs.iter().any(|i| i.size == size) {
                specs.push(IntegerLayout {
                    size,
                    abi_alignment,
                    preferred_alignment,
                });
            }
        }

        specs.sort();
        specs
    }

    /// Augments the parsed pointer specifications with any missing information
    /// based on the defaults for LLVM's data layout.
    fn pointer_specs_or_defaults(mut specs: Vec<PointerLayout>) -> Vec<PointerLayout> {
        let pointer_layouts = [DEFAULT_POINTER_0_LAYOUT];

        for (space, size, abi, pref, index) in pointer_layouts {
            if !specs.iter().any(|l| l.address_space == space) {
                specs.push(PointerLayout {
                    address_space: space,
                    size,
                    abi_alignment: abi,
                    preferred_alignment: pref,
                    index_size: index,
                });
            }
        }

        specs.sort();
        specs
    }
}

impl Default for DataLayout {
    fn default() -> Self {
        Self::new("").expect("The empty string was not a valid data layout specification")
    }
}

impl TryFrom<&str> for DataLayout {
    type Error = Error;

    fn try_from(value: &str) -> std::result::Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<String> for DataLayout {
    type Error = Error;

    fn try_from(value: String) -> std::result::Result<Self, Self::Error> {
        Self::new(&value)
    }
}

/// A description of the endianness used when laying out data.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Endianness {
    /// Little-endian (least-significant byte first).
    Little,

    /// Big-endian (most-significant byte first).
    Big,
}

impl Endianness {
    /// Parses the endianness specification part of the data-layout.
    fn parser() -> impl parsing::DLParser<Endianness> {
        choice((
            just("e").to(Endianness::Little),
            just("E").to(Endianness::Big),
        ))
    }
}

/// A description of the mangling scheme used by this module.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Mangling {
    /// The Unix COFF mangling scheme that is still used by Windows' Portable
    /// Executable format.
    ///
    /// Private symbols get the usual prefix. Functions with `__stdcall`,
    /// `__fastcall`, and `__vectorcall` have custom mangling that appends
    /// `@N` where `N` is the number of bytes used to pass parameters. C++
    /// symbols starting with `?` are not mangled in any way.
    COFF,

    /// The Windows x86 COFF mangling scheme.
    ///
    /// Private symbols get the usual prefix. Regular C symbols get an `_`
    /// prefix. Functions with `__stdcall`, `__fastcall`, and `__vectorcall`
    /// have custom mangling that appends `@N` where `N` is the number of
    /// bytes used to pass parameters. C++ symbols starting with `?` are not
    /// mangled in any way.
    COFF86,

    /// The ELF mangling scheme, where private symbols get a `.L` prefix.
    ELF,

    /// The GOFF mangling scheme, where private symbols get an `@` prefix.
    GOFF,

    /// The Mach-O mangling scheme, where private symbols get an `L` prefix and
    /// other symbols get an `_` prefix.
    MachO,

    /// The MIPS mangling scheme, where private symbols get a `$` prefix.
    MIPS,

    /// The XCOFF mangling scheme, where private symbols get an `L..` prefix.
    XCOFF,
}

impl Mangling {
    /// Parses the mangling specification part of the data-layout.
    fn parser() -> impl parsing::DLParser<Mangling> {
        just("m:").ignore_then(choice((
            just("a").to(Mangling::XCOFF),
            just("e").to(Mangling::ELF),
            just("l").to(Mangling::GOFF),
            just("m").to(Mangling::MIPS),
            just("o").to(Mangling::MachO),
            just("w").to(Mangling::COFF),
            just("x").to(Mangling::COFF86),
        )))
    }
}

/// A specification of the pointer layout for this data-layout.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct PointerLayout {
    /// The address space for which the pointer is being specified.
    pub address_space: usize,

    /// The size of the pointer.
    pub size: usize,

    /// The required ABI alignment for the pointer.
    pub abi_alignment: usize,

    /// The preferred alignment for the pointer.
    pub preferred_alignment: usize,

    /// The size of the index used for address calculation.
    pub index_size: usize,
}

impl PointerLayout {
    /// Parses the pointer layout specification as part of the data layout
    /// string.
    #[must_use]
    pub fn parser() -> impl parsing::DLParser<PointerLayout> {
        just("p")
            .ignore_then(parsing::pos_int(10).delimited_by(just("["), just("]")).or_not())
            .then(parsing::field(parsing::pos_int(10)))
            .then(parsing::field(parsing::pos_int(10)))
            .then(parsing::field(parsing::pos_int(10)).or_not())
            .then(parsing::field(parsing::pos_int(10)).or_not())
            .try_map(
                |((((address_space, size), abi_alignment), preferred_alignment), index_size),
                 span| {
                    let address_space = address_space.unwrap_or(0);
                    let preferred_alignment = preferred_alignment.unwrap_or(abi_alignment);
                    let index_size = index_size.unwrap_or(size);
                    if index_size > size {
                        Err(Simple::custom(
                            span,
                            format!(
                                "The requested index size {index_size} is larger than the pointer \
                                 size {size}"
                            ),
                        ))?;
                    };

                    Ok(Self {
                        address_space,
                        size,
                        abi_alignment,
                        preferred_alignment,
                        index_size,
                    })
                },
            )
    }
}

/// A specification of an integer layout for this data-layout.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct IntegerLayout {
    /// The size of the integer.
    pub size: usize,

    /// The required ABI alignment for the integer.
    pub abi_alignment: usize,

    /// The preferred alignment for the integer.
    pub preferred_alignment: usize,
}

impl IntegerLayout {
    /// Parses an integer layout specification as part of the data layout
    /// string.
    #[must_use]
    pub fn parser() -> impl parsing::DLParser<IntegerLayout> {
        just("i")
            .ignore_then(parsing::pos_int(10))
            .then(parsing::field(parsing::pos_int(10)))
            .then(parsing::field(parsing::pos_int(10)).or_not())
            .try_map(|((size, abi_alignment), preferred_alignment), span| {
                let preferred_alignment = preferred_alignment.unwrap_or(abi_alignment);
                if size == BYTE_SIZE && abi_alignment != size {
                    Err(Simple::custom(
                        span,
                        "i8 was not aligned to a byte boundary",
                    ))?;
                }

                Ok(Self {
                    size,
                    abi_alignment,
                    preferred_alignment,
                })
            })
    }
}

/// A specification of a vector layout for this data-layout.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct VectorLayout {
    /// The size of the vector.
    pub size: usize,

    /// The required ABI alignment for the vector.
    pub abi_alignment: usize,

    /// The preferred alignment for the vector.
    pub preferred_alignment: usize,
}

impl VectorLayout {
    /// Parses a vector layout specification as part of the data layout
    /// string.
    #[must_use]
    pub fn parser() -> impl parsing::DLParser<VectorLayout> {
        just("v")
            .ignore_then(parsing::pos_int(10))
            .then(parsing::field(parsing::pos_int(10)))
            .then(parsing::field(parsing::pos_int(10)).or_not())
            .map(|((size, abi_alignment), preferred_alignment)| {
                let preferred_alignment = preferred_alignment.unwrap_or(abi_alignment);

                Self {
                    size,
                    abi_alignment,
                    preferred_alignment,
                }
            })
    }
}

/// A specification of a floating-point layout for this data-layout.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct FloatLayout {
    /// The size of the floating-point number.
    pub size: usize,

    /// The required ABI alignment for the floating-point number.
    pub abi_alignment: usize,

    /// The preferred alignment for the floating-point number.
    pub preferred_alignment: usize,
}

impl FloatLayout {
    /// Parses a floating-point layout specification as part of the data layout
    /// string.
    #[must_use]
    pub fn parser() -> impl parsing::DLParser<FloatLayout> {
        just("f")
            .ignore_then(parsing::pos_int(10))
            .then(parsing::field(parsing::pos_int(10)))
            .then(parsing::field(parsing::pos_int(10)).or_not())
            .try_map(|((size, abi_alignment), preferred_alignment), span| {
                let preferred_alignment = preferred_alignment.unwrap_or(abi_alignment);
                if !&[16, 32, 64, 80, 128].contains(&size) {
                    Err(Simple::custom(
                        span,
                        format!("{size} is not a valid floating-point size"),
                    ))?;
                }

                Ok(Self {
                    size,
                    abi_alignment,
                    preferred_alignment,
                })
            })
    }
}

/// A specification of the aggregate layout for this data-layout.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct AggregateLayout {
    /// The required ABI alignment for an aggregate.
    pub abi_alignment: usize,

    /// The preferred alignment for an aggregate.
    pub preferred_alignment: usize,
}

impl AggregateLayout {
    /// Parses the aggregate layout specification as part of the data layout
    /// string.
    #[must_use]
    pub fn parser() -> impl parsing::DLParser<AggregateLayout> {
        just("a")
            .ignore_then(parsing::pos_int(10))
            .then(parsing::field(parsing::pos_int(10)).or_not())
            .map(|(abi_alignment, preferred_alignment)| {
                let preferred_alignment = preferred_alignment.unwrap_or(abi_alignment);

                Self {
                    abi_alignment,
                    preferred_alignment,
                }
            })
    }
}

/// A specification of the way function pointers are treated as part of this
/// data-layout.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum FunctionPointerType {
    /// The alignment of function pointers is independent of the alignment of
    /// functions, and is a multiple of the ABI alignment.
    Independent,

    /// The alignment of function pointers is a multiple of the explicit
    /// alignment specified on the function, and is a multiple of the ABI
    /// alignment.
    Multiple,
}

impl FunctionPointerType {
    /// Parses the function pointer type as part of the data layout string.
    #[must_use]
    pub fn parser() -> impl parsing::DLParser<FunctionPointerType> {
        choice((
            just("i").to(FunctionPointerType::Independent),
            just("n").to(FunctionPointerType::Multiple),
        ))
    }
}

/// A specification of the function pointer layout for this data-layout.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct FunctionPointerLayout {
    /// The way that the function pointer is treated in the data layout.
    pub ptr_type: FunctionPointerType,

    /// The alignment of function pointers in this data layout.
    pub abi_alignment: usize,
}

impl FunctionPointerLayout {
    /// Parses the function pointer layout specification as part of this data
    /// layout.
    #[must_use]
    pub fn parser() -> impl parsing::DLParser<FunctionPointerLayout> {
        just("F")
            .ignore_then(FunctionPointerType::parser())
            .then(parsing::pos_int(10))
            .map(|(ptr_type, abi_alignment)| Self {
                ptr_type,
                abi_alignment,
            })
    }
}

/// A specification of the native integer widths for this data-layout.
///
/// The CPU must have _at least one_ native integer width.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct NativeIntegerWidths {
    /// The integer widths that are natively supported on the CPU.
    pub widths: Vec<usize>,
}

impl NativeIntegerWidths {
    /// Parses the specification of native integer widths for the target CPU.
    #[must_use]
    pub fn parser() -> impl parsing::DLParser<NativeIntegerWidths> {
        just("n")
            .ignore_then(parsing::pos_int(10))
            .then(parsing::field(parsing::pos_int(10)).repeated())
            .map(|(first, mut rest)| {
                rest.insert(0, first);
                Self { widths: rest }
            })
    }
}

/// A specification of the address spaces in which the pointers should be
/// treated as [non-integral](https://llvm.org/docs/LangRef.html#nointptrtype).
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct NonIntegralPointerAddressSpaces {
    /// The address spaces in which pointers should be treated as non-integral.
    pub address_spaces: Vec<usize>,
}

impl NonIntegralPointerAddressSpaces {
    /// Parses the specification of address-spaces in which pointers are
    /// non-integral.
    #[must_use]
    pub fn parser() -> impl parsing::DLParser<NonIntegralPointerAddressSpaces> {
        just("ni")
            .ignore_then(parsing::field(parsing::pos_int(10)).repeated().at_least(1))
            .try_map(|address_spaces, span| {
                if address_spaces.contains(&0) {
                    Err(Simple::custom(
                        span,
                        "The 0 address space cannot be specified as using non-integral pointers",
                    ))?;
                }

                Ok(Self { address_spaces })
            })
    }
}

/// Utility parsing functions to aid in the parsing of data-layouts but that are
/// not associated directly with any type.
pub mod parsing {
    use chumsky::{error::Simple, prelude::just, text::int, Parser};

    use crate::{constant::BYTE_SIZE, llvm::data_layout::parsing};

    /// Simply to avoid typing out the whole parser type parameter specification
    /// every single time given it only varies in one parameter.
    pub trait DLParser<T>: Parser<char, T, Error = Simple<char>> {}

    /// A blanket impl to make this work, because yay.
    impl<T, U> DLParser<T> for U where U: Parser<char, T, Error = Simple<char>> {}

    /// Parses an element separator.
    #[must_use]
    pub fn elem_sep<'a>() -> impl DLParser<&'a str> {
        just("-")
    }

    /// Parses a field separator.
    #[must_use]
    pub fn field_sep<'a>() -> impl DLParser<&'a str> {
        just(":")
    }

    /// Parses a field, namely a colon followed by something as given by the
    /// `then` parser.
    pub fn field<T>(then: impl DLParser<T>) -> impl DLParser<T> {
        field_sep().ignore_then(then)
    }

    /// Parses a positive integer in the specified `radix`.
    #[must_use]
    pub fn pos_int(radix: u32) -> impl DLParser<usize> {
        int(radix).try_map(|num: String, span| {
            num.parse::<usize>().map_err(|_| {
                Simple::custom(span, format!("Could not parse {num} as a positive integer"))
            })
        })
    }

    /// Parses the stack alignment specification part of the data-layout.
    #[must_use]
    pub fn stack_alignment() -> impl DLParser<usize> {
        just("S").ignore_then(pos_int(10)).validate(|alignment, span, emit| {
            if alignment % BYTE_SIZE != 0 {
                emit(Simple::custom(
                    span,
                    format!("{alignment} must be aligned to a byte offset"),
                ));
            }
            alignment
        })
    }

    /// Parses the address space specification part of the data-layout.
    fn address_space(space: &str) -> impl DLParser<usize> + '_ {
        just(space).ignore_then(parsing::pos_int(10))
    }

    #[must_use]
    pub fn program_address_space() -> impl DLParser<usize> {
        address_space("P")
    }

    #[must_use]
    pub fn global_address_space() -> impl DLParser<usize> {
        address_space("G")
    }

    #[must_use]
    pub fn alloc_address_space() -> impl DLParser<usize> {
        address_space("A")
    }
}

#[cfg(test)]
mod test {
    use chumsky::Parser;

    use crate::llvm::data_layout::{
        parsing,
        AggregateLayout,
        DataLayout,
        Endianness,
        FloatLayout,
        FunctionPointerLayout,
        FunctionPointerType,
        IntegerLayout,
        Mangling,
        NativeIntegerWidths,
        NonIntegralPointerAddressSpaces,
        PointerLayout,
        VectorLayout,
    };

    #[test]
    fn can_parse_data_layout() {
        let dl_string = "e-m:e-i8:8:32-i16:16:32-i64:64-i128:128-n32:64-S128";

        // It should parse correctly
        let parsed_layout = DataLayout::new(dl_string);
        assert!(parsed_layout.is_ok());

        // Now we can check that the fields have their proper values.
        let layout = parsed_layout.unwrap();

        // Little endian with ELF mangling
        assert_eq!(layout.endianness, Endianness::Little);
        assert_eq!(layout.mangling, Mangling::ELF);

        // Stack aligned to 128 bits, with all address spaces in zero.
        assert_eq!(layout.stack_alignment, 128);
        assert_eq!(layout.program_address_space, 0);
        assert_eq!(layout.global_address_space, 0);
        assert_eq!(layout.alloc_address_space, 0);

        // Pointers in address space zero are aligned to 64 bits.
        assert_eq!(
            layout.pointer_layouts,
            vec![PointerLayout {
                address_space:       0,
                size:                64,
                abi_alignment:       64,
                preferred_alignment: 64,
                index_size:          64,
            }]
        );

        // Integers are semi-customized, with 8, 16, 64, and 128 using layouts specified
        // in the string
        assert!(layout.integer_layouts.contains(&IntegerLayout {
            size:                1,
            abi_alignment:       8,
            preferred_alignment: 8,
        }));
        assert!(layout.integer_layouts.contains(&IntegerLayout {
            size:                8,
            abi_alignment:       8,
            preferred_alignment: 32,
        }));
        assert!(layout.integer_layouts.contains(&IntegerLayout {
            size:                16,
            abi_alignment:       16,
            preferred_alignment: 32,
        }));
        assert!(layout.integer_layouts.contains(&IntegerLayout {
            size:                32,
            abi_alignment:       32,
            preferred_alignment: 32,
        }));
        assert!(layout.integer_layouts.contains(&IntegerLayout {
            size:                64,
            abi_alignment:       64,
            preferred_alignment: 64,
        }));
        assert!(layout.integer_layouts.contains(&IntegerLayout {
            size:                128,
            abi_alignment:       128,
            preferred_alignment: 128,
        }));

        // For vector layouts we only have the defaults
        assert!(layout.vector_layouts.contains(&VectorLayout {
            size:                64,
            abi_alignment:       64,
            preferred_alignment: 64,
        }));
        assert!(layout.vector_layouts.contains(&VectorLayout {
            size:                128,
            abi_alignment:       128,
            preferred_alignment: 128,
        }));

        // For float layouts we also use the defaults
        assert!(layout.float_layouts.contains(&FloatLayout {
            size:                16,
            abi_alignment:       16,
            preferred_alignment: 16,
        }));
        assert!(layout.float_layouts.contains(&FloatLayout {
            size:                32,
            abi_alignment:       32,
            preferred_alignment: 32,
        }));
        assert!(layout.float_layouts.contains(&FloatLayout {
            size:                64,
            abi_alignment:       64,
            preferred_alignment: 64,
        }));
        assert!(layout.float_layouts.contains(&FloatLayout {
            size:                128,
            abi_alignment:       128,
            preferred_alignment: 128,
        }));

        // For the aggregate layout we have the default
        assert_eq!(
            layout.aggregate_layout,
            AggregateLayout {
                abi_alignment:       0,
                preferred_alignment: 64,
            }
        );

        // For the function pointer layout we also have our default
        assert_eq!(
            layout.function_pointer_layout,
            FunctionPointerLayout {
                ptr_type:      FunctionPointerType::Independent,
                abi_alignment: 64,
            }
        );

        // For native integer widths this string specifies 32, 64
        assert_eq!(
            layout.native_integer_widths,
            NativeIntegerWidths {
                widths: vec![32, 64],
            }
        );

        // And no address spaces should be using non-integral pointers
        assert_eq!(
            layout.nointptr_address_spaces,
            NonIntegralPointerAddressSpaces {
                address_spaces: Vec::new(),
            }
        );
    }

    #[test]
    fn can_parse_data_layout_to_default() {
        let dl_string = "";

        // It should parse correctly
        let parsed_layout = DataLayout::new(dl_string);
        assert!(parsed_layout.is_ok());

        // Now we can check that the fields have their proper values.
        let layout = parsed_layout.unwrap();

        // Little endian with ELF mangling
        assert_eq!(layout.endianness, Endianness::Little);
        assert_eq!(layout.mangling, Mangling::ELF);

        // Stack alignment is arbitrary, with all address spaces in zero.
        assert_eq!(layout.stack_alignment, 0);
        assert_eq!(layout.program_address_space, 0);
        assert_eq!(layout.global_address_space, 0);
        assert_eq!(layout.alloc_address_space, 0);

        // Pointers in address space zero are aligned to 64 bits.
        assert_eq!(
            layout.pointer_layouts,
            vec![PointerLayout {
                address_space:       0,
                size:                64,
                abi_alignment:       64,
                preferred_alignment: 64,
                index_size:          64,
            }]
        );

        // All the integer layouts should be default
        assert!(layout.integer_layouts.contains(&IntegerLayout {
            size:                1,
            abi_alignment:       8,
            preferred_alignment: 8,
        }));
        assert!(layout.integer_layouts.contains(&IntegerLayout {
            size:                8,
            abi_alignment:       8,
            preferred_alignment: 8,
        }));
        assert!(layout.integer_layouts.contains(&IntegerLayout {
            size:                16,
            abi_alignment:       16,
            preferred_alignment: 16,
        }));
        assert!(layout.integer_layouts.contains(&IntegerLayout {
            size:                32,
            abi_alignment:       32,
            preferred_alignment: 32,
        }));
        assert!(layout.integer_layouts.contains(&IntegerLayout {
            size:                64,
            abi_alignment:       32,
            preferred_alignment: 64,
        }));

        // For vector layouts we only have the defaults
        assert!(layout.vector_layouts.contains(&VectorLayout {
            size:                64,
            abi_alignment:       64,
            preferred_alignment: 64,
        }));
        assert!(layout.vector_layouts.contains(&VectorLayout {
            size:                128,
            abi_alignment:       128,
            preferred_alignment: 128,
        }));

        // For float layouts we also use the defaults
        assert!(layout.float_layouts.contains(&FloatLayout {
            size:                16,
            abi_alignment:       16,
            preferred_alignment: 16,
        }));
        assert!(layout.float_layouts.contains(&FloatLayout {
            size:                32,
            abi_alignment:       32,
            preferred_alignment: 32,
        }));
        assert!(layout.float_layouts.contains(&FloatLayout {
            size:                64,
            abi_alignment:       64,
            preferred_alignment: 64,
        }));
        assert!(layout.float_layouts.contains(&FloatLayout {
            size:                128,
            abi_alignment:       128,
            preferred_alignment: 128,
        }));

        // For the aggregate layout we have the default
        assert_eq!(
            layout.aggregate_layout,
            AggregateLayout {
                abi_alignment:       0,
                preferred_alignment: 64,
            }
        );

        // For the function pointer layout we also have our default
        assert_eq!(
            layout.function_pointer_layout,
            FunctionPointerLayout {
                ptr_type:      FunctionPointerType::Independent,
                abi_alignment: 64,
            }
        );

        // For native integer widths we should have the default
        assert_eq!(
            layout.native_integer_widths,
            NativeIntegerWidths {
                widths: vec![32, 64],
            }
        );

        // And no address spaces should be using non-integral pointers
        assert_eq!(
            layout.nointptr_address_spaces,
            NonIntegralPointerAddressSpaces {
                address_spaces: Vec::new(),
            }
        );
    }

    #[test]
    fn can_parse_endianness_segment() {
        // Failures
        assert!(Endianness::parser().parse("foo").is_err());

        // Successes
        assert_eq!(
            Endianness::parser()
                .parse("e")
                .expect("Little endian spec did not parse"),
            Endianness::Little
        );
        assert_eq!(
            Endianness::parser()
                .parse("E")
                .expect("Big endian spec did not parse"),
            Endianness::Big
        );
    }

    #[test]
    fn can_parse_mangling_segment() {
        // Failures
        assert!(Mangling::parser().parse("m:").is_err());
        assert!(Mangling::parser().parse("m:f").is_err());
        assert!(Mangling::parser().parse("f").is_err());

        // Successes
        assert_eq!(Mangling::parser().parse("m:a"), Ok(Mangling::XCOFF));
        assert_eq!(Mangling::parser().parse("m:e"), Ok(Mangling::ELF));
        assert_eq!(Mangling::parser().parse("m:l"), Ok(Mangling::GOFF));
        assert_eq!(Mangling::parser().parse("m:m"), Ok(Mangling::MIPS));
        assert_eq!(Mangling::parser().parse("m:o"), Ok(Mangling::MachO));
        assert_eq!(Mangling::parser().parse("m:w"), Ok(Mangling::COFF));
        assert_eq!(Mangling::parser().parse("m:x"), Ok(Mangling::COFF86));
    }

    #[test]
    fn can_parse_stack_alignment_segment() {
        // Failures
        assert!(parsing::stack_alignment().parse("m:").is_err());
        assert!(parsing::stack_alignment().parse("S").is_err());
        assert!(parsing::stack_alignment().parse("S15").is_err());

        // Successes
        assert_eq!(parsing::stack_alignment().parse("S8"), Ok(8));
        assert_eq!(parsing::stack_alignment().parse("S32"), Ok(32));
        assert_eq!(parsing::stack_alignment().parse("S64"), Ok(64));
        assert_eq!(parsing::stack_alignment().parse("S128"), Ok(128));
        assert_eq!(parsing::stack_alignment().parse("S256"), Ok(256));
    }

    #[test]
    fn can_parse_program_address_space() {
        // Failures
        assert!(parsing::program_address_space().parse("PA").is_err());
        assert!(parsing::program_address_space().parse("P").is_err());

        // Successes
        assert_eq!(parsing::program_address_space().parse("P1"), Ok(1));
        assert_eq!(parsing::program_address_space().parse("P0"), Ok(0));
    }

    #[test]
    fn can_parse_global_address_space() {
        // Failures
        assert!(parsing::global_address_space().parse("GA").is_err());
        assert!(parsing::global_address_space().parse("G").is_err());

        // Successes
        assert_eq!(parsing::global_address_space().parse("G1"), Ok(1));
        assert_eq!(parsing::global_address_space().parse("G0"), Ok(0));
    }

    #[test]
    fn can_parse_alloc_address_space() {
        // Failures
        assert!(parsing::alloc_address_space().parse("AA").is_err());
        assert!(parsing::alloc_address_space().parse("A").is_err());

        // Successes
        assert_eq!(parsing::alloc_address_space().parse("A1"), Ok(1));
        assert_eq!(parsing::alloc_address_space().parse("A0"), Ok(0));
    }

    #[test]
    fn can_parse_pointer_spec() {
        // Failures
        assert!(PointerLayout::parser().parse("p[1]:64:128:128:68").is_err());
        assert!(PointerLayout::parser().parse("p[]:64:128:128:32").is_err());

        // Successes
        assert_eq!(
            PointerLayout::parser().parse("p[1]:64:128:128:64"),
            Ok(PointerLayout {
                address_space:       1,
                size:                64,
                abi_alignment:       128,
                preferred_alignment: 128,
                index_size:          64,
            })
        );
        assert_eq!(
            PointerLayout::parser().parse("p:64:128:128:64"),
            Ok(PointerLayout {
                address_space:       0,
                size:                64,
                abi_alignment:       128,
                preferred_alignment: 128,
                index_size:          64,
            })
        );
        assert_eq!(
            PointerLayout::parser().parse("p:64:128"),
            Ok(PointerLayout {
                address_space:       0,
                size:                64,
                abi_alignment:       128,
                preferred_alignment: 128,
                index_size:          64,
            })
        );
    }

    #[test]
    fn can_parse_integer_spec() {
        // Failures
        assert!(IntegerLayout::parser().parse("i").is_err());
        assert!(IntegerLayout::parser().parse("i8:16").is_err());

        // Successes
        assert_eq!(
            IntegerLayout::parser().parse("i8:8"),
            Ok(IntegerLayout {
                size:                8,
                abi_alignment:       8,
                preferred_alignment: 8,
            })
        );
        assert_eq!(
            IntegerLayout::parser().parse("i32:64"),
            Ok(IntegerLayout {
                size:                32,
                abi_alignment:       64,
                preferred_alignment: 64,
            })
        );
        assert_eq!(
            IntegerLayout::parser().parse("i32:64:128"),
            Ok(IntegerLayout {
                size:                32,
                abi_alignment:       64,
                preferred_alignment: 128,
            })
        );
    }

    #[test]
    fn can_parse_vector_spec() {
        // Failures
        assert!(VectorLayout::parser().parse("v").is_err());
        assert!(VectorLayout::parser().parse("v8").is_err());

        // Successes
        assert_eq!(
            VectorLayout::parser().parse("v8:8"),
            Ok(VectorLayout {
                size:                8,
                abi_alignment:       8,
                preferred_alignment: 8,
            })
        );
        assert_eq!(
            VectorLayout::parser().parse("v32:64"),
            Ok(VectorLayout {
                size:                32,
                abi_alignment:       64,
                preferred_alignment: 64,
            })
        );
        assert_eq!(
            VectorLayout::parser().parse("v32:64:128"),
            Ok(VectorLayout {
                size:                32,
                abi_alignment:       64,
                preferred_alignment: 128,
            })
        );
    }

    #[test]
    fn can_parse_float_spec() {
        // Failures
        assert!(FloatLayout::parser().parse("f").is_err());
        assert!(FloatLayout::parser().parse("f8:16").is_err());
        assert!(FloatLayout::parser().parse("f96:128").is_err());

        // Successes
        assert_eq!(
            FloatLayout::parser().parse("f16:16"),
            Ok(FloatLayout {
                size:                16,
                abi_alignment:       16,
                preferred_alignment: 16,
            })
        );
        assert_eq!(
            FloatLayout::parser().parse("f32:64"),
            Ok(FloatLayout {
                size:                32,
                abi_alignment:       64,
                preferred_alignment: 64,
            })
        );
        assert_eq!(
            FloatLayout::parser().parse("f32:64:128"),
            Ok(FloatLayout {
                size:                32,
                abi_alignment:       64,
                preferred_alignment: 128,
            })
        );
    }

    #[test]
    fn can_parse_aggregate_spec() {
        // Failures
        assert!(FloatLayout::parser().parse("a").is_err());

        // Successes
        assert_eq!(
            AggregateLayout::parser().parse("a64"),
            Ok(AggregateLayout {
                abi_alignment:       64,
                preferred_alignment: 64,
            })
        );
        assert_eq!(
            AggregateLayout::parser().parse("a64:128"),
            Ok(AggregateLayout {
                abi_alignment:       64,
                preferred_alignment: 128,
            })
        );
    }

    #[test]
    fn can_parse_function_pointer_type() {
        // Failures
        assert!(FunctionPointerType::parser().parse("a").is_err());

        // Successes
        assert_eq!(
            FunctionPointerType::parser().parse("i"),
            Ok(FunctionPointerType::Independent)
        );
        assert_eq!(
            FunctionPointerType::parser().parse("n"),
            Ok(FunctionPointerType::Multiple)
        );
    }

    #[test]
    fn can_parse_function_pointer_spec() {
        // Failures
        assert!(FunctionPointerLayout::parser().parse("Fi").is_err());
        assert!(FunctionPointerLayout::parser().parse("Fb64").is_err());

        // Successes
        assert_eq!(
            FunctionPointerLayout::parser().parse("Fi64"),
            Ok(FunctionPointerLayout {
                ptr_type:      FunctionPointerType::Independent,
                abi_alignment: 64,
            })
        );
        assert_eq!(
            FunctionPointerLayout::parser().parse("Fi128"),
            Ok(FunctionPointerLayout {
                ptr_type:      FunctionPointerType::Independent,
                abi_alignment: 128,
            })
        );
        assert_eq!(
            FunctionPointerLayout::parser().parse("Fn32"),
            Ok(FunctionPointerLayout {
                ptr_type:      FunctionPointerType::Multiple,
                abi_alignment: 32,
            })
        );
    }

    #[test]
    fn can_parse_native_integer_widths_spec() {
        // Failures
        assert!(NativeIntegerWidths::parser().parse("Fi").is_err());
        assert!(NativeIntegerWidths::parser().parse("n").is_err());

        // Successes
        assert_eq!(
            NativeIntegerWidths::parser().parse("n64"),
            Ok(NativeIntegerWidths { widths: vec![64] })
        );
        assert_eq!(
            NativeIntegerWidths::parser().parse("n16:32:64"),
            Ok(NativeIntegerWidths {
                widths: vec![16, 32, 64],
            })
        );
    }

    #[test]
    fn can_parse_nointptr_address_spaces_spec() {
        // Failures
        assert!(NonIntegralPointerAddressSpaces::parser().parse("ni").is_err());
        assert!(NonIntegralPointerAddressSpaces::parser().parse("ni:").is_err());
        assert!(NonIntegralPointerAddressSpaces::parser().parse("ni:0").is_err());

        // Successes
        assert_eq!(
            NonIntegralPointerAddressSpaces::parser().parse("ni:1"),
            Ok(NonIntegralPointerAddressSpaces {
                address_spaces: vec![1],
            })
        );
        assert_eq!(
            NonIntegralPointerAddressSpaces::parser().parse("ni:1:3:5"),
            Ok(NonIntegralPointerAddressSpaces {
                address_spaces: vec![1, 3, 5],
            })
        );
    }
}
