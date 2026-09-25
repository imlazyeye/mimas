use macros::native;
use rand::RngExt;
use shared::Ty;
use vm::api::Api;

pub(crate) fn install<'gc>(api: &mut Api<'_, 'gc>) {
    api.add_assoc(Ty::Bool, random);
}

/// Returns `true` or `false`, each with equal chance.
///
/// ```mimas
/// let side = if bool::random() "heads" else "tails";
/// ```
#[native]
fn random() -> bool {
    rand::rng().random()
}
