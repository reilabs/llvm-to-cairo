//! The compiler's internal representation of LLVM types, without being tied to
//! the context as the [`BasicTypeEnum`] is.

use std::fmt::{Display, Formatter};

use inkwell::types::{BasicTypeEnum, FunctionType};
use itertools::Itertools;
use ltc_errors::{compile, compile::Error};

use crate::constant::BYTE_SIZE;

/// A representation of the LLVM [types](https://llvm.org/docs/LangRef.html#type-system)
/// for use within the compiler.
///
/// # Why Not Use BasicTypeEnum?
///
/// The definition of Inkwell's [`BasicTypeEnum`] depends on being tied directly
/// to the LLVM context, which is not something we want for metadata that is
/// likely to be passed around liberally within this compiler. To that end, we
/// convert it to our own internal representation with the knowledge that this
/// static and does not update if the internal LLVM representation changes.
///
/// # Value Semantics
///
/// It is intended that this type is used as having value semantics, and not
/// ever have a reference returned to it.
#[derive(Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
pub enum LLVMType {
    /// The boolean type, represented inside LLVM by the `i1`
    /// [integer type](https://llvm.org/docs/LangRef.html#integer-type).
    #[allow(non_camel_case_types)] // To better match the LLVM internal convention
    bool,

    /// The 8-bit wide [integer type](https://llvm.org/docs/LangRef.html#integer-type).
    #[allow(non_camel_case_types)] // To better match the LLVM internal convention
    i8,

    /// The 16-bit wide [integer type](https://llvm.org/docs/LangRef.html#integer-type).
    #[allow(non_camel_case_types)] // To better match the LLVM internal convention
    i16,

    /// The 32-bit wide [integer type](https://llvm.org/docs/LangRef.html#integer-type).
    #[allow(non_camel_case_types)] // To better match the LLVM internal convention
    i32,

    /// The 64-bit wide [integer type](https://llvm.org/docs/LangRef.html#integer-type).
    #[allow(non_camel_case_types)] // To better match the LLVM internal convention
    i64,

    /// The 128-bit wide [integer type](https://llvm.org/docs/LangRef.html#integer-type).
    #[allow(non_camel_case_types)] // To better match the LLVM internal convention
    i128,

    /// The IEEE-754 `binary16` [floating point type](https://llvm.org/docs/LangRef.html#floating-point-types).
    #[allow(non_camel_case_types)] // To better match the LLVM internal convention
    half,

    /// The IEEE-754 `binary32` [floating point type](https://llvm.org/docs/LangRef.html#floating-point-types).
    #[allow(non_camel_case_types)] // To better match the LLVM internal convention
    float,

    /// The IEEE-754 `binary64` [floating point type](https://llvm.org/docs/LangRef.html#floating-point-types).
    #[allow(non_camel_case_types)] // To better match the LLVM internal convention
    double,

    /// Used to specify locations in memory as described in the
    /// [LLVM IR reference](https://llvm.org/docs/LangRef.html#pointer-type).
    ///
    /// Note that pointers in our use only support the base address space, and
    /// do not specify the corresponding pointee type as was available in
    /// earlier versions of LLVM.
    #[allow(non_camel_case_types)] // To better match the LLVM internal convention
    ptr,

    /// A [type](https://llvm.org/docs/LangRef.html#void-type) that does not
    /// represent any value and has no size.
    #[allow(non_camel_case_types)] // To better match the LLVM internal convention
    void,

    /// An [array](https://llvm.org/docs/LangRef.html#array-type) is a
    /// sequential arrangement of a number of elements of the given type
    /// linearly in memory.
    Array {
        /// The number of elements in the array type.
        count: usize,

        /// The type of elements in the array type.
        ty: Box<LLVMType>,
    },

    /// A [structure](https://llvm.org/docs/LangRef.html#structure-type)
    /// represents a number of elements together in memory.
    ///
    /// Note that struct elements do not have names, and can only be accessed by
    /// index. This makes these struct types far more akin to a tuple.
    Structure {
        /// If the structure is packed, it has one-byte alignment with no
        /// padding between elements.
        ///
        /// If it is not packed, then the padding and alignment of struct
        /// elements is given by the module's data-layout string.
        packed: bool,

        /// The element types in the structure type.
        ///
        /// The order is semantically meaninful here.
        elements: Vec<LLVMType>,
    },

    /// A [function](https://llvm.org/docs/LangRef.html#function-type) is akin
    /// to a function signature.
    Function {
        /// The type returned from the function.
        return_type: Box<LLVMType>,

        /// The types of the parameters to the function.
        ///
        /// Note that these are never named, and are purely matched
        /// positionally.
        parameter_types: Vec<LLVMType>,
    },

    /// Embedded [metadata](https://llvm.org/docs/LangRef.html#metadata-type)
    /// used as a value has this type.
    Metadata,
}

/// Additional utility constructors for creating the compound types without
/// having to manage boxing manually.
impl LLVMType {
    /// Builds an array type containing the provided `elem_count` number of
    /// elements of type `elem_type`.
    pub fn make_array(elem_count: usize, elem_type: LLVMType) -> Self {
        Self::Array {
            count: elem_count,
            ty:    Box::new(elem_type),
        }
    }

    /// Creates a struct type from the provided `elem_types` and whether it is
    /// `packed`.
    pub fn make_struct(packed: bool, elem_types: &[LLVMType]) -> Self {
        Self::Structure {
            packed,
            elements: Vec::from(elem_types),
        }
    }

    /// Creates a function type from the provided `return_type` and
    /// `param_types`.
    pub fn make_function(return_type: LLVMType, param_types: &[LLVMType]) -> Self {
        Self::Function {
            return_type:     Box::new(return_type),
            parameter_types: Vec::from(param_types),
        }
    }
}

/// Operations for working with LLVM types, such as asserting properties on
/// them, or processing them.
impl LLVMType {
    /// Checks if the LLVM type represented by `self` unifies with the type
    /// represented by `other`.
    pub fn unifies(&self, other: LLVMType) -> bool {
        self == &other
    }

    /// Returns `true` if `self` is a primitive type, and `false` otherwise.
    pub fn is_prim(&self) -> bool {
        matches!(
            self,
            Self::bool
                | Self::i8
                | Self::i32
                | Self::i64
                | Self::i128
                | Self::half
                | Self::float
                | Self::double
                | Self::ptr
                | Self::void
                | Self::Metadata
        )
    }

    /// Returns `true` if `self` is a compound type, and `false` otherwise.
    pub fn is_compound(&self) -> bool {
        !self.is_prim()
    }

    /// Returns `true` if `self` is an integral type, and `false` otherwise.
    pub fn is_integral(&self) -> bool {
        matches!(
            self,
            Self::bool | Self::i8 | Self::i16 | Self::i32 | Self::i64 | Self::i128
        )
    }

    /// Returns `true` if `self` is a floating-point type, and `false`
    /// otherwise.
    pub fn is_float(&self) -> bool {
        matches!(self, Self::half | Self::float | Self::double)
    }
}

impl Display for LLVMType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let result = match self {
            LLVMType::bool => "bool".to_string(),
            LLVMType::i8 => "i8".to_string(),
            LLVMType::i16 => "i16".to_string(),
            LLVMType::i32 => "i32".to_string(),
            LLVMType::i64 => "i64".to_string(),
            LLVMType::i128 => "i128".to_string(),
            LLVMType::half => "half".to_string(),
            LLVMType::float => "float".to_string(),
            LLVMType::double => "double".to_string(),
            LLVMType::ptr => "ptr".to_string(),
            LLVMType::void => "void".to_string(),
            LLVMType::Metadata => "metadata".to_string(),
            LLVMType::Array { count, ty } => {
                let ty_str = ty.to_string();
                format!("[{ty_str}; {count}]")
            }
            LLVMType::Structure { packed, elements } => {
                let elem_strs = elements.iter().map(|e| e.to_string()).join(", ");
                if *packed {
                    format!("<{{ {elem_strs} }}>")
                } else {
                    format!("{{ {elem_strs} }}")
                }
            }
            LLVMType::Function {
                return_type,
                parameter_types,
            } => {
                let params_string = parameter_types.iter().map(|e| e.to_string()).join(", ");
                format!("({params_string}) -> {return_type}")
            }
        };

        writeln!(f, "{result}")
    }
}

impl<'ctx> TryFrom<BasicTypeEnum<'ctx>> for LLVMType {
    type Error = compile::Error;

    fn try_from(value: BasicTypeEnum<'ctx>) -> Result<Self, Self::Error> {
        Self::try_from(&value)
    }
}

impl<'ctx> TryFrom<&BasicTypeEnum<'ctx>> for LLVMType {
    type Error = compile::Error;

    fn try_from(value: &BasicTypeEnum<'ctx>) -> Result<Self, Self::Error> {
        let result_type = match value {
            BasicTypeEnum::ArrayType(array_type) => {
                let length = array_type.len() as usize;
                let elem_type = Self::try_from(array_type.get_element_type())?;
                Self::make_array(length, elem_type)
            }
            BasicTypeEnum::FloatType(float_type) => {
                let float_size_bits = float_type
                    .size_of()
                    .get_sign_extended_constant()
                    .ok_or(Error::UnsupportedType(value.to_string()))?
                    * BYTE_SIZE;
                match float_size_bits {
                    16 => Self::half,
                    32 => Self::float,
                    64 => Self::double,
                    _ => Err(Error::UnsupportedType(value.to_string()))?,
                }
            }
            BasicTypeEnum::IntType(int_type) => match int_type.get_bit_width() {
                1 => Self::bool,
                8 => Self::i8,
                16 => Self::i16,
                32 => Self::i32,
                64 => Self::i64,
                128 => Self::i128,
                _ => Err(Error::UnsupportedType(value.to_string()))?,
            },
            BasicTypeEnum::PointerType(_) => Self::ptr,
            BasicTypeEnum::StructType(struct_type) => {
                let field_types: Vec<Self> = struct_type
                    .get_field_types()
                    .iter()
                    .map(|t| Self::try_from(t))
                    .collect::<Result<Vec<Self>, Error>>()?;
                let packed = struct_type.is_packed();
                Self::make_struct(packed, &field_types)
            }
            BasicTypeEnum::VectorType(_) => Err(Error::UnsupportedType(value.to_string()))?,
        };

        Ok(result_type)
    }
}

impl<'ctx> TryFrom<FunctionType<'ctx>> for LLVMType {
    type Error = compile::Error;

    fn try_from(value: FunctionType<'ctx>) -> Result<Self, Self::Error> {
        Self::try_from(&value)
    }
}

impl<'ctx> TryFrom<&FunctionType<'ctx>> for LLVMType {
    type Error = compile::Error;

    fn try_from(value: &FunctionType<'ctx>) -> Result<Self, Self::Error> {
        let return_type = value
            .get_return_type()
            .map(|ty| Self::try_from(ty))
            .unwrap_or(Ok(LLVMType::void))?;
        let param_types = value
            .get_param_types()
            .iter()
            .map(|ty| Self::try_from(ty))
            .collect::<Result<Vec<Self>, Error>>()?;

        Ok(Self::make_function(return_type, &param_types))
    }
}

/// A trait representing objects that have an [`LLVMType`] ascribed to them.
pub trait HasLLVMType {
    /// Gets the LLVM type for the implementing object.
    fn get_type(&self) -> LLVMType;
}

impl HasLLVMType for LLVMType {
    fn get_type(&self) -> LLVMType {
        self.clone()
    }
}
