use crate::function_sig::FnSig;
use crate::r#type::tycache::TyCache;
use crate::r#type::{DotnetTypeRef, Type};
use rustc_middle::mir::{Operand, UnOp};
use rustc_middle::ty::{Instance, TyCtxt, TyKind,UintTy,IntTy};

use crate::cil::{CILOp, CallSite};
pub fn unop<'ctx>(
    unnop: UnOp,
    operand: &Operand<'ctx>,
    tcx: TyCtxt<'ctx>,
    method: &rustc_middle::mir::Body<'ctx>,
    method_instance: Instance<'ctx>,
    tycache: &mut TyCache,
) -> Vec<CILOp> {
    let mut ops = crate::operand::handle_operand(operand, tcx, method, method_instance, tycache);
    let ty = operand.ty(&method.local_decls, tcx);
    match unnop {
        UnOp::Neg => match ty.kind() {
            TyKind::Int(IntTy::I128) => ops.push(CILOp::Call(CallSite::boxed(
                DotnetTypeRef::int_128().into(),
                "op_UnaryNegation".into(),
                FnSig::new(&[Type::I128], &Type::I128),
                true,
            ))),
            _ => ops.push(CILOp::Neg),
        },
        UnOp::Not => match ty.kind() {
            TyKind::Bool => ops.extend([CILOp::LdcI32(0), CILOp::Eq]),
            TyKind::Uint(UintTy::U128) => ops.push(CILOp::Call(CallSite::boxed(
                DotnetTypeRef::uint_128().into(),
                "op_OnesComplement".into(),
                FnSig::new(&[Type::U128], &Type::U128),
                true,
            ))),
            TyKind::Int(IntTy::I128) => ops.push(CILOp::Call(CallSite::boxed(
                DotnetTypeRef::int_128().into(),
                "op_OnesComplement".into(),
                FnSig::new(&[Type::I128], &Type::I128),
                true,
            ))),
            //TyKind::U128 => ops.extend([CILOp::LdcI32(0), CILOp::Eq]),
            _ => ops.push(CILOp::Not),
        },
    };
    ops
}
