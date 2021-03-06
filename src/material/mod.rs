use crate::interaction::SurfaceInteraction;
use bumpalo::Bump;
use crate::reflection::bsdf::Bsdf;

pub mod matte;
pub mod mirror;
pub mod glass;
pub mod metal;
pub mod plastic;

#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub enum TransportMode {
    Radiance,
    Importance,
}

pub trait Material: Sync + Send {
    fn compute_scattering_functions<'a>(
        &self,
        si: &SurfaceInteraction,
        arena: &'a Bump,
        mode: TransportMode,
        allow_multiple_lobes: bool
    ) -> Bsdf<'a>;
}