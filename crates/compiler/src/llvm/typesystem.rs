//! The compiler's internal representation of LLVM types, without being tied to
//! the context as the [`BasicTypeEnum`] is.

use std::fmt::{Display, Formatter};

use inkwell::types::{
    AnyTypeEnum,
    ArrayType,
    BasicTypeEnum,
    FloatType,
    FunctionType,
    IntType,
    PointerType,
    StructType,
    VectorType,
    VoidType,
};
use itertools::Itertools;
use ltc_errors::{compile, compile::Error};

use crate::constant::BYTE_SIZE;

/// A representation of the LLVM [types](https://llvm.org/docs/LangRef.html#type-system)
/// for use within the compiler.
///
/// # Why Not Use `BasicTypeEnum`?
///
/// The definition of Inkwell's [`BasicTypeEnum`] and [`AnyTypeEnum`] depends on
/// being tied directly to the host LLVM context. This is not something we want
/// for metadata that is likely to be passed around liberally within this
/// compiler and potentially even cross program boundaries.
///
/// To that end, we convert it to our own internal representation with the
/// knowledge that this static and does not update if the internal LLVM
/// representation changes.
///
/// We additionally want to restrict the allowable types in our use-case. This
/// enum **does not** match LLVM IR's type system 1:1, instead restricting the
/// allowable types—particularly the integers—to be the ones that we care about.
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
    /// index. This makes LLVM struct types far more akin to what we call a
    /// Tuple in most languages.
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
    #[must_use]
    pub fn make_array(elem_count: usize, elem_type: LLVMType) -> Self {
        Self::Array {
            count: elem_count,
            ty:    Box::new(elem_type),
        }
    }

    /// Creates a struct type from the provided `elem_types` and whether it is
    /// `packed`.
    #[must_use]
    pub fn make_struct(packed: bool, elem_types: &[LLVMType]) -> Self {
        Self::Structure {
            packed,
            elements: Vec::from(elem_types),
        }
    }

    /// Creates a function type from the provided `return_type` and
    /// `param_types`.
    #[must_use]
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
    ///
    /// Please note that this is currently purely an equality check. It exists
    /// so that in the future we can seamlessly implement more complex
    /// unification rules if needed.
    #[must_use]
    pub fn unifies_with(&self, other: &LLVMType) -> bool {
        self == other
    }

    /// Returns `true` if `self` is a primitive type, and `false` otherwise.
    #[must_use]
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
    #[must_use]
    pub fn is_compound(&self) -> bool {
        !self.is_prim()
    }

    /// Returns `true` if `self` is an integral type, and `false` otherwise.
    #[must_use]
    pub fn is_integral(&self) -> bool {
        matches!(
            self,
            Self::bool | Self::i8 | Self::i16 | Self::i32 | Self::i64 | Self::i128
        )
    }

    /// Returns `true` if `self` is a floating-point type, and `false`
    /// otherwise.
    #[must_use]
    pub fn is_float(&self) -> bool {
        matches!(self, Self::half | Self::float | Self::double)
    }
}

/// This attempts to match the LLVM representations for these types where it is
/// reasonable.
///
/// For Array types we currently use the Rust syntax as that is clearer to read
/// than the LLVM product-style syntax.
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
                let elem_strs = elements.iter().map(std::string::ToString::to_string).join(", ");
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
                let params_string = parameter_types
                    .iter()
                    .map(std::string::ToString::to_string)
                    .join(", ");
                format!("({params_string}) -> {return_type}")
            }
        };

        writeln!(f, "{result}")
    }
}

/// Conversion from Inkwell's generic type enum to our type language.
impl<'ctx> TryFrom<AnyTypeEnum<'ctx>> for LLVMType {
    type Error = compile::Error;

    fn try_from(value: AnyTypeEnum<'ctx>) -> Result<Self, Self::Error> {
        Self::try_from(&value)
    }
}

/// Conversion from Inkwell's generic type enum to our type language.
impl<'ctx> TryFrom<&AnyTypeEnum<'ctx>> for LLVMType {
    type Error = compile::Error;

    fn try_from(value: &AnyTypeEnum<'ctx>) -> Result<Self, Self::Error> {
        match value {
            AnyTypeEnum::ArrayType(array_type) => Self::try_from(array_type),
            AnyTypeEnum::FloatType(float_type) => Self::try_from(float_type),
            AnyTypeEnum::FunctionType(fn_ty) => Self::try_from(fn_ty),
            AnyTypeEnum::IntType(int_type) => Self::try_from(int_type),
            AnyTypeEnum::PointerType(ptr_type) => Self::try_from(ptr_type),
            AnyTypeEnum::StructType(struct_type) => Self::try_from(struct_type),
            AnyTypeEnum::VoidType(void_type) => Self::try_from(void_type),
            AnyTypeEnum::VectorType(vector_type) => Self::try_from(vector_type),
        }
    }
}

/// Conversion from Inkwell's basic type enum to our type language.
impl<'ctx> TryFrom<BasicTypeEnum<'ctx>> for LLVMType {
    type Error = compile::Error;

    fn try_from(value: BasicTypeEnum<'ctx>) -> Result<Self, Self::Error> {
        Self::try_from(&value)
    }
}

/// Conversion from Inkwell's basic type enum to our type language.
impl<'ctx> TryFrom<&BasicTypeEnum<'ctx>> for LLVMType {
    type Error = compile::Error;

    fn try_from(value: &BasicTypeEnum<'ctx>) -> Result<Self, Self::Error> {
        match value {
            BasicTypeEnum::ArrayType(array_type) => Self::try_from(array_type),
            BasicTypeEnum::FloatType(float_type) => Self::try_from(float_type),
            BasicTypeEnum::IntType(int_type) => Self::try_from(int_type),
            BasicTypeEnum::PointerType(ptr_type) => Self::try_from(ptr_type),
            BasicTypeEnum::StructType(struct_type) => Self::try_from(struct_type),
            BasicTypeEnum::VectorType(vector_type) => Self::try_from(vector_type),
        }
    }
}

/// Conversion from Inkwell's array type to our type language.
impl<'ctx> TryFrom<ArrayType<'ctx>> for LLVMType {
    type Error = compile::Error;

    fn try_from(value: ArrayType<'ctx>) -> Result<Self, Self::Error> {
        Self::try_from(&value)
    }
}

/// Conversion from Inkwell's array type to our type language.
impl<'ctx> TryFrom<&ArrayType<'ctx>> for LLVMType {
    type Error = compile::Error;

    fn try_from(value: &ArrayType<'ctx>) -> Result<Self, Self::Error> {
        let length = value.len() as usize;
        let elem_type = Self::try_from(value.get_element_type())?;
        Ok(Self::make_array(length, elem_type))
    }
}

/// Conversion from Inkwell's generic float type to our specific float types.
impl<'ctx> TryFrom<FloatType<'ctx>> for LLVMType {
    type Error = compile::Error;

    fn try_from(value: FloatType<'ctx>) -> Result<Self, Self::Error> {
        Self::try_from(&value)
    }
}

/// Conversion from Inkwell's generic float type to our specific float types.
impl<'ctx> TryFrom<&FloatType<'ctx>> for LLVMType {
    type Error = compile::Error;

    fn try_from(value: &FloatType<'ctx>) -> Result<Self, Self::Error> {
        #[allow(clippy::cast_possible_wrap)] // Our byte size should never be large enough
        let float_size_bits = value
            .size_of()
            .get_sign_extended_constant()
            .ok_or(Error::UnsupportedType(value.to_string()))?
            * BYTE_SIZE as i64;
        let ret_val = match float_size_bits {
            16 => Self::half,
            32 => Self::float,
            64 => Self::double,
            _ => Err(Error::UnsupportedType(value.to_string()))?,
        };
        Ok(ret_val)
    }
}

/// Conversion from Inkwell's generic integer type to our specific integer
/// types.
impl<'ctx> TryFrom<IntType<'ctx>> for LLVMType {
    type Error = compile::Error;

    fn try_from(value: IntType<'ctx>) -> Result<Self, Self::Error> {
        Self::try_from(&value)
    }
}

/// Conversion from Inkwell's generic integer type to our specific integer
/// types.
impl<'ctx> TryFrom<&IntType<'ctx>> for LLVMType {
    type Error = compile::Error;

    fn try_from(value: &IntType<'ctx>) -> Result<Self, Self::Error> {
        let res = match value.get_bit_width() {
            1 => Self::bool,
            8 => Self::i8,
            16 => Self::i16,
            32 => Self::i32,
            64 => Self::i64,
            128 => Self::i128,
            _ => Err(Error::UnsupportedType(value.to_string()))?,
        };

        Ok(res)
    }
}

/// Conversion from Inkwell's pointer type to our type language.
///
/// We centralize it here despite it being trivial as this gives us one place to
/// potentially need to change if we ever add type system support for typed
/// pointers. Otherwise, we would have to change every site performing
/// conversion of pointer types.
impl<'ctx> TryFrom<PointerType<'ctx>> for LLVMType {
    type Error = compile::Error;

    fn try_from(value: PointerType<'ctx>) -> Result<Self, Self::Error> {
        Self::try_from(&value)
    }
}

/// Conversion from Inkwell's pointer type to our type language.
///
/// We centralize it here despite it being trivial as this gives us one place to
/// potentially need to change if we ever add type system support for typed
/// pointers. Otherwise, we would have to change every site performing
/// conversion of pointer types.
impl<'ctx> TryFrom<&PointerType<'ctx>> for LLVMType {
    type Error = compile::Error;

    fn try_from(_: &PointerType<'ctx>) -> Result<Self, Self::Error> {
        Ok(Self::ptr)
    }
}

/// Conversion from Inkwell's struct type to our type language.
impl<'ctx> TryFrom<StructType<'ctx>> for LLVMType {
    type Error = compile::Error;

    fn try_from(value: StructType<'ctx>) -> Result<Self, Self::Error> {
        Self::try_from(&value)
    }
}

/// Conversion from Inkwell's struct type to our type language.
impl<'ctx> TryFrom<&StructType<'ctx>> for LLVMType {
    type Error = compile::Error;

    fn try_from(value: &StructType<'ctx>) -> Result<Self, Self::Error> {
        let field_types: Vec<Self> = value
            .get_field_types()
            .iter()
            .map(Self::try_from)
            .collect::<Result<Vec<Self>, Error>>()?;
        let packed = value.is_packed();
        Ok(Self::make_struct(packed, &field_types))
    }
}

/// Conversion from Inkwell's vector type to our type language.
///
/// Currently, our type language **cannot represent** the SIMD vector types, so
/// this operation will error. It exists to ensure that in the future we can
/// seamlessly add support without having to change multiple conversion sites
/// that would currently need to produce errors.
impl<'ctx> TryFrom<VectorType<'ctx>> for LLVMType {
    type Error = compile::Error;

    fn try_from(value: VectorType<'ctx>) -> Result<Self, Self::Error> {
        Self::try_from(&value)
    }
}

/// Conversion from Inkwell's vector type to our type language.
///
/// Currently, our type language **cannot represent** the SIMD vector types, so
/// this operation will error. It exists to ensure that in the future we can
/// seamlessly add support without having to change multiple conversion sites
/// that would currently need to produce errors.
impl<'ctx> TryFrom<&VectorType<'ctx>> for LLVMType {
    type Error = compile::Error;

    fn try_from(value: &VectorType<'ctx>) -> Result<Self, Self::Error> {
        Err(Error::UnsupportedType(value.to_string()))?
    }
}

/// Conversion from Inkwell's function type to our type language.
impl<'ctx> TryFrom<FunctionType<'ctx>> for LLVMType {
    type Error = compile::Error;

    fn try_from(value: FunctionType<'ctx>) -> Result<Self, Self::Error> {
        Self::try_from(&value)
    }
}

/// Conversion from Inkwell's function type to our type language.
impl<'ctx> TryFrom<&FunctionType<'ctx>> for LLVMType {
    type Error = compile::Error;

    fn try_from(value: &FunctionType<'ctx>) -> Result<Self, Self::Error> {
        let return_type = value.get_return_type().map_or(Ok(LLVMType::void), Self::try_from)?;
        let param_types = value
            .get_param_types()
            .iter()
            .map(Self::try_from)
            .collect::<Result<Vec<Self>, Error>>()?;

        Ok(Self::make_function(return_type, &param_types))
    }
}

/// Conversion from Inkwell's void type to our type language.
///
/// We centralize this in a conversion to ensure that it is consistent at all
/// use sites.
impl<'ctx> TryFrom<VoidType<'ctx>> for LLVMType {
    type Error = compile::Error;

    fn try_from(value: VoidType<'ctx>) -> Result<Self, Self::Error> {
        Self::try_from(&value)
    }
}

/// Conversion from Inkwell's void type to our type language.
///
/// We centralize this in a conversion to ensure that it is consistent at all
/// use sites.
impl<'ctx> TryFrom<&VoidType<'ctx>> for LLVMType {
    type Error = compile::Error;

    fn try_from(_: &VoidType<'ctx>) -> Result<Self, Self::Error> {
        Ok(Self::void)
    }
}
