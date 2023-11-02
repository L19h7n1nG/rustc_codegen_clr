use rustc_middle::mir::{BinOp, Operand};
use rustc_middle::ty::{Instance, TyCtxt};

use crate::cil_op::{CILOp, FieldDescriptor};
use crate::r#type::{DotnetTypeRef, Type};
/// Preforms an checked binary operation.
pub(crate) fn binop_checked<'tcx>(
    binop: BinOp,
    operand_a: &Operand<'tcx>,
    operand_b: &Operand<'tcx>,
    tcx: TyCtxt<'tcx>,
    method: &rustc_middle::mir::Body<'tcx>,
    method_instance: Instance<'tcx>,
) -> Vec<CILOp> {
    let ops_a = crate::operand::handle_operand(operand_a, tcx, method, method_instance);
    let ops_b = crate::operand::handle_operand(operand_b, tcx, method, method_instance);
    let ty_a = operand_a.ty(&method.local_decls, tcx);
    let ty_b = operand_b.ty(&method.local_decls, tcx);
    assert_eq!(ty_a, ty_b);
    let ty = Type::from_ty(ty_a, tcx, &method_instance);
    match binop {
        BinOp::Mul | BinOp::MulUnchecked => [ops_a, ops_b, mul(ty).into()]
            .into_iter()
            .flatten()
            .collect(),
        BinOp::Add => [ops_a, ops_b, add(ty)].into_iter().flatten().collect(),
        BinOp::Sub => [ops_a, ops_b, sub(ty).into()]
            .into_iter()
            .flatten()
            .collect(),
        _ => todo!("Can't preform checked op {binop:?}"),
    }
}
fn mul(tpe: Type) -> Vec<CILOp> {
    match tpe {
        Type::U8 => promoted_ubinop(
            Type::U8,
            Type::U16,
            CILOp::ConvU16(false),
            CILOp::ConvU8(false),
            CILOp::LdcI32(u8::MAX as i32),
            CILOp::Mul,
        ),
        Type::I8 => promoted_sbinop(
            Type::I8,
            Type::I16,
            CILOp::ConvI16(false),
            CILOp::ConvI8(false),
            CILOp::LdcI32(i8::MAX as i32),
            CILOp::LdcI32(i8::MIN as i32),
            CILOp::Mul,
        ),
        _ => todo!("Can't preform checked mul on type {tpe:?} yet!"),
    }
}
fn add(tpe: Type) -> Vec<CILOp> {
    match tpe {
        Type::I8 => promoted_sbinop(
            Type::I8,
            Type::I16,
            CILOp::ConvI16(false),
            CILOp::ConvI8(false),
            CILOp::LdcI32(i8::MAX as i32),
            CILOp::LdcI32(i8::MIN as i32),
            CILOp::Add,
        ),
        Type::U8 => checked_uadd_type(Type::U8, CILOp::ConvU8(false)),
        Type::I16 => promoted_sbinop(
            Type::I16,
            Type::I32,
            CILOp::ConvI32(false),
            CILOp::ConvI16(false),
            CILOp::LdcI32(i16::MAX as i32),
            CILOp::LdcI32(i16::MIN as i32),
            CILOp::Add,
        ),
        Type::U16 => checked_uadd_type(Type::U16, CILOp::ConvU16(false)),
        Type::I32 => promoted_sbinop(
            Type::I32,
            Type::I64,
            CILOp::ConvI64(false),
            CILOp::ConvI32(false),
            CILOp::LdcI32(i32::MAX),
            CILOp::LdcI32(i32::MIN),
            CILOp::Add,
        ),
        Type::U32 => checked_uadd_type(Type::U32, CILOp::Nop),
        //This works ONLY in dotnet.
        Type::I64 => promoted_sbinop(
            Type::I64,
            Type::I128,
            CILOp::Call(crate::cil_op::CallSite::boxed(
                Some(DotnetTypeRef::int_128()),
                "op_Implicit".into(),
                crate::function_sig::FnSig::new(&[Type::I64], &Type::I128),
                true,
            )),
            CILOp::Call(crate::cil_op::CallSite::boxed(
                Some(DotnetTypeRef::int_128()),
                "op_Explicit".into(),
                crate::function_sig::FnSig::new(&[Type::I128], &Type::I64),
                true,
            )),
            CILOp::LdcI64(i64::MAX),
            CILOp::LdcI64(i64::MIN),
            CILOp::Call(crate::cil_op::CallSite::boxed(
                Some(DotnetTypeRef::int_128()),
                "op_Addition".into(),
                crate::function_sig::FnSig::new(&[Type::I128, Type::I128], &Type::I128),
                true,
            )),
        ),
        Type::U64 => checked_uadd_type(Type::U64, CILOp::Nop),
        _ => todo!("Can't preform checked add on type {tpe:?} yet!"),
    }
}
fn checked_uadd_type(tpe: Type, truncate: CILOp) -> Vec<CILOp> {
    let tuple = crate::r#type::simple_tuple(&[tpe.clone(), Type::Bool]);
    let tuple_ty = tuple.clone().into();
    vec![
        CILOp::NewTMPLocal(tpe.clone().into()),
        CILOp::SetTMPLocal,
        CILOp::NewTMPLocal(tpe.clone().into()),
        CILOp::SetTMPLocal,
        CILOp::LoadTMPLocal,
        CILOp::LoadUnderTMPLocal(1),
        CILOp::Add,
        truncate,
        CILOp::Dup,
        CILOp::NewTMPLocal(tpe.clone().into()),
        CILOp::SetTMPLocal,
        CILOp::LoadUnderTMPLocal(1),
        CILOp::LoadUnderTMPLocal(2),
        CILOp::Or,
        CILOp::Lt,
        CILOp::NewTMPLocal(Type::Bool.into()),
        CILOp::SetTMPLocal,
        CILOp::NewTMPLocal(Box::new(tuple_ty)),
        CILOp::LoadAddresOfTMPLocal,
        CILOp::LoadUnderTMPLocal(1),
        CILOp::STField(FieldDescriptor::boxed(
            tuple.clone(),
            Type::GenericArg(1),
            "Item2".into(),
        )),
        CILOp::LoadAddresOfTMPLocal,
        CILOp::LoadUnderTMPLocal(2),
        CILOp::STField(FieldDescriptor::boxed(
            tuple.clone(),
            Type::GenericArg(0),
            "Item1".into(),
        )),
        CILOp::LoadTMPLocal,
        CILOp::FreeTMPLocal,
        CILOp::FreeTMPLocal,
        CILOp::FreeTMPLocal,
        CILOp::FreeTMPLocal,
        CILOp::FreeTMPLocal,
    ]
}
fn checked_sadd_type(tpe: Type, truncate: CILOp, mask: CILOp) -> Vec<CILOp> {
    let tuple = crate::r#type::simple_tuple(&[tpe.clone(), Type::Bool]);
    let tuple_ty: Type = tuple.clone().into();
    vec![
        CILOp::NewTMPLocal(tpe.clone().into()),
        CILOp::SetTMPLocal,
        CILOp::NewTMPLocal(tpe.clone().into()),
        CILOp::SetTMPLocal,
        CILOp::LoadTMPLocal,
        CILOp::LoadUnderTMPLocal(1),
        CILOp::Add,
        truncate.clone(),
        CILOp::NewTMPLocal(tpe.clone().into()),
        CILOp::SetTMPLocal,
        CILOp::LoadUnderTMPLocal(1),
        mask.clone(),
        CILOp::And,
        CILOp::Dup,
        CILOp::LoadUnderTMPLocal(2),
        mask.clone(),
        CILOp::And,
        CILOp::Eq,
        truncate.clone(),
        CILOp::NewTMPLocal(tpe.clone().into()),
        CILOp::SetTMPLocal,
        CILOp::LoadUnderTMPLocal(1),
        mask.clone(),
        CILOp::And,
        CILOp::Eq,
        CILOp::Not,
        CILOp::LoadTMPLocal,
        CILOp::And,
        CILOp::NewTMPLocal(tpe.clone().into()),
        CILOp::SetTMPLocal,
        CILOp::NewTMPLocal(tuple_ty.into()),
        CILOp::LoadAddresOfTMPLocal,
        CILOp::LoadUnderTMPLocal(1),
        CILOp::STField(FieldDescriptor::boxed(
            tuple.clone(),
            Type::GenericArg(1),
            "Item2".into(),
        )),
        CILOp::LoadAddresOfTMPLocal,
        CILOp::LoadUnderTMPLocal(3),
        CILOp::STField(FieldDescriptor::boxed(
            tuple.clone(),
            Type::GenericArg(0),
            "Item1".into(),
        )),
        CILOp::LoadTMPLocal,
        CILOp::FreeTMPLocal,
        CILOp::FreeTMPLocal,
        CILOp::FreeTMPLocal,
        CILOp::FreeTMPLocal,
        CILOp::FreeTMPLocal,
        CILOp::FreeTMPLocal,
    ]
}
fn sub(tpe: Type) -> Vec<CILOp> {
    match tpe {
        Type::I8 => promoted_sbinop(
            Type::I8,
            Type::I16,
            CILOp::ConvI16(false),
            CILOp::ConvI8(false),
            CILOp::LdcI32(i8::MAX as i32),
            CILOp::LdcI32(i8::MIN as i32),
            CILOp::Sub,
        ),
        Type::U8 => promoted_ubinop(
            Type::U8,
            Type::U16,
            CILOp::ConvU16(false),
            CILOp::ConvU8(false),
            CILOp::LdcI32(u8::MAX as i32),
            CILOp::Sub,
        ),
        Type::I16 => promoted_sbinop(
            Type::I16,
            Type::I32,
            CILOp::ConvI32(false),
            CILOp::ConvI16(false),
            CILOp::LdcI32(i16::MAX as i32),
            CILOp::LdcI32(i16::MIN as i32),
            CILOp::Sub,
        ),
        Type::U16 => promoted_ubinop(
            Type::U16,
            Type::U32,
            CILOp::ConvU32(false),
            CILOp::ConvU16(false),
            CILOp::LdcI32(u16::MAX as i32),
            CILOp::Sub,
        ),
        Type::I32 => promoted_sbinop(
            Type::I32,
            Type::I64,
            CILOp::ConvI64(false),
            CILOp::ConvI32(false),
            CILOp::LdcI32(i32::MAX),
            CILOp::LdcI32(i32::MIN),
            CILOp::Sub,
        ),
        Type::U32 => promoted_ubinop(
            Type::U32,
            Type::U64,
            CILOp::ConvU64(false),
            CILOp::ConvU32(false),
            CILOp::LdcI64(u64::MAX as i64),
            CILOp::Sub,
        ),
        //This works ONLY in dotnet.
        Type::I64 => promoted_sbinop(
            Type::I64,
            Type::I128,
            CILOp::Call(crate::cil_op::CallSite::boxed(
                Some(DotnetTypeRef::int_128()),
                "op_Implicit".into(),
                crate::function_sig::FnSig::new(&[Type::I64], &Type::I128),
                true,
            )),
            CILOp::Call(crate::cil_op::CallSite::boxed(
                Some(DotnetTypeRef::int_128()),
                "op_Explicit".into(),
                crate::function_sig::FnSig::new(&[Type::I128], &Type::I64),
                true,
            )),
            CILOp::LdcI64(i64::MAX),
            CILOp::LdcI64(i64::MIN),
            CILOp::Call(crate::cil_op::CallSite::boxed(
                Some(DotnetTypeRef::int_128()),
                "op_Subtraction".into(),
                crate::function_sig::FnSig::new(&[Type::I128, Type::I128], &Type::I128),
                true,
            )),
        ),
        Type::U64 => promoted_ubinop(
            Type::U64,
            Type::U128,
            CILOp::Call(crate::cil_op::CallSite::boxed(
                Some(DotnetTypeRef::uint_128()),
                "op_Implicit".into(),
                crate::function_sig::FnSig::new(&[Type::U64], &Type::U128),
                true,
            )),
            CILOp::Call(crate::cil_op::CallSite::boxed(
                Some(DotnetTypeRef::uint_128()),
                "op_Explicit".into(),
                crate::function_sig::FnSig::new(&[Type::U128], &Type::U64),
                true,
            )),
            CILOp::LdcI64(i64::MIN),
            CILOp::Call(crate::cil_op::CallSite::boxed(
                Some(DotnetTypeRef::uint_128()),
                "op_Subtraction".into(),
                crate::function_sig::FnSig::new(&[Type::U128, Type::U128], &Type::U128),
                true,
            )),
        ),
        _ => todo!("Can't preform checked sub on type {tpe:?} yet!"),
    }
}
#[test]
fn unsigned_add() {
    //u8
    for a in 0..u8::MAX {
        for b in 0..u8::MAX {
            let added = u8::checked_add(a, b);
            let alg_added = {
                let sum = u8::wrapping_add(a, b);
                if sum < a | b {
                    None
                } else {
                    Some(sum)
                }
            };
            assert_eq!(added, alg_added, "checked {a} + {b}");
        }
    }
}
#[test]
fn signed_add() {
    //u8
    for a in i8::MIN..i8::MAX {
        for b in i8::MIN..i8::MAX {
            let added = i8::checked_add(a, b);
            let alg_added = {
                let sum = i8::wrapping_add(a, b);
                let sign_a = a & (0b1000_0000_u8 as i8);
                let sign_b = b & (0b1000_0000_u8 as i8);
                let sum_sign = sum & (0b1000_0000_u8 as i8);
                let signs_equal = sign_a == sign_b;
                if signs_equal && sum_sign != sign_a {
                    None
                } else {
                    Some(sum)
                }
            };
            assert_eq!(added, alg_added, "checked {a} + {b}");
        }
    }
}
pub fn promoted_ubinop(
    tpe: Type,
    promoted_type: Type,
    promote: CILOp,
    truncate: CILOp,
    omask: CILOp,
    binop: CILOp,
) -> Vec<CILOp> {
    let tuple = crate::r#type::simple_tuple(&[tpe.clone(), Type::Bool]);
    let tuple_ty: Type = tuple.clone().into();
    vec![
        // Promote arguments
        CILOp::NewTMPLocal(tpe.clone().into()),
        CILOp::SetTMPLocal,
        promote.clone(),
        CILOp::LoadTMPLocal,
        promote.clone(),
        // Preform binop
        binop.clone(),
        // Save the promoted result of binop
        CILOp::NewTMPLocal(promoted_type.clone().into()),
        CILOp::SetTMPLocal,
        // Compare the result to the overflow mask
        CILOp::LoadTMPLocal,
        omask.clone(),
        promote.clone(),
        CILOp::Gt,
        // Save the bollean indicating overflow
        CILOp::NewTMPLocal(Type::Bool.into()),
        CILOp::SetTMPLocal,
        // Create result tuple type
        CILOp::NewTMPLocal(tuple_ty.into()),
        // Set the tuples second field to overflow flag
        CILOp::LoadAddresOfTMPLocal,
        CILOp::LoadUnderTMPLocal(1), // ov
        CILOp::STField(FieldDescriptor::boxed(
            tuple.clone(),
            Type::GenericArg(1),
            "Item2".into(),
        )),
        // Set the tuples first field to promotion result
        CILOp::LoadAddresOfTMPLocal,
        CILOp::LoadUnderTMPLocal(2),
        truncate,
        CILOp::STField(FieldDescriptor::boxed(
            tuple.clone(),
            Type::GenericArg(0),
            "Item1".into(),
        )),
        // Load results
        CILOp::LoadTMPLocal,
        // Reset temporary local statck.
        CILOp::FreeTMPLocal,
        CILOp::FreeTMPLocal,
        CILOp::FreeTMPLocal,
        CILOp::FreeTMPLocal,
    ]
}
pub fn promoted_sbinop(
    tpe: Type,
    promoted_type: Type,
    promote: CILOp,
    truncate: CILOp,
    omask: CILOp,
    umask: CILOp,
    binop: CILOp,
) -> Vec<CILOp> {
    let tuple = crate::r#type::simple_tuple(&[tpe.clone(), Type::Bool]);
    let tuple_ty: Type = tuple.clone().into();
    vec![
        // Promote arguments
        CILOp::NewTMPLocal(tpe.clone().into()),
        CILOp::SetTMPLocal,
        promote.clone(),
        CILOp::LoadTMPLocal,
        promote.clone(),
        // Preform binop
        binop.clone(),
        // Save the promoted result of binop
        CILOp::NewTMPLocal(promoted_type.clone().into()),
        CILOp::SetTMPLocal,
        // Compare the result to the overflow mask
        CILOp::LoadTMPLocal,
        omask.clone(),
        promote.clone(),
        CILOp::Gt,
        CILOp::LoadTMPLocal,
        // Compare the result to the undeflow mask
        umask.clone(),
        promote.clone(),
        CILOp::Lt,
        CILOp::Or,
        // Save the bollean indicating overflow
        CILOp::NewTMPLocal(Type::Bool.into()),
        CILOp::SetTMPLocal,
        // Create result tuple type
        CILOp::NewTMPLocal(tuple_ty.into()),
        // Set the tuples second field to overflow flag
        CILOp::LoadAddresOfTMPLocal,
        CILOp::LoadUnderTMPLocal(1), // ov
        CILOp::STField(FieldDescriptor::boxed(
            tuple.clone(),
            Type::GenericArg(1),
            "Item2".into(),
        )),
        // Set the tuples first field to promotion result
        CILOp::LoadAddresOfTMPLocal,
        CILOp::LoadUnderTMPLocal(2),
        truncate,
        CILOp::STField(FieldDescriptor::boxed(
            tuple.clone(),
            Type::GenericArg(0),
            "Item1".into(),
        )),
        // Load results
        CILOp::LoadTMPLocal,
        // Reset temporary local statck.
        CILOp::FreeTMPLocal,
        CILOp::FreeTMPLocal,
        CILOp::FreeTMPLocal,
        CILOp::FreeTMPLocal,
    ]
}
