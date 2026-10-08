use macros::native;
use rand::RngExt;
use shared::Ty;
use vm::{Ctx, api::Api};

use crate::Random;

pub(crate) fn install<'gc>(api: &mut Api<'_, 'gc>) {
    api.add_assoc(Ty::Bool, random);
}

/// Returns `true` or `false`, each with equal chance.
///
/// ```mimas
/// let side = if bool::random() "heads" else "tails";
/// ```
#[native]
fn random<'gc>(ctx: Ctx<'gc>) -> bool {
    ctx.fixture::<Random>().rng().random()
}
