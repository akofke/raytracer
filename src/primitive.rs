use std::sync::Arc;

use crate::{Ray, SurfaceInteraction};
use crate::geometry::bounds::Bounds3f;
use crate::material::Material;
use crate::shapes::Shape;
use crate::light::{AreaLight, Light};
use crate::spectrum::Spectrum;
use crate::light::diffuse::DiffuseAreaLight;

pub trait Primitive: Sync {
    fn world_bound(&self) -> Bounds3f;

    fn intersect(&self, ray: &mut Ray) -> Option<SurfaceInteraction>;

    fn intersect_test(&self, ray: &Ray) -> bool;

    fn material(&self) -> Option<&dyn Material>;

    fn area_light(&self) -> Option<&dyn AreaLight>;
    
    fn light_arc_cloned(&self) -> Option<Arc<dyn Light>>;
}

pub struct GeometricPrimitive<S: Shape> {
    pub shape: Arc<S>,
    pub material: Option<Arc<dyn Material>>,
    pub light: Option<Arc<DiffuseAreaLight<S>>>,
}

impl<S: Shape> GeometricPrimitive<S> {
    pub fn set_emitter(&mut self, emit: Spectrum, n_samples: usize) {
        // FIXME: transform
        let light = DiffuseAreaLight::new(
            emit,
            self.shape.clone(),
            n_samples,
        );
        self.light = Some(Arc::new(light))
    }
}

impl<S: 'static +  Shape> Primitive for GeometricPrimitive<S> {
    fn world_bound(&self) -> Bounds3f {
        self.shape.world_bound()
    }

    fn intersect(&self, ray: &mut Ray) -> Option<SurfaceInteraction> {
        let (t_hit, mut intersect) = self.shape.intersect(ray)?;

        ray.t_max = t_hit;
        intersect.primitive = Some(self); // TODO: this is terrible
        Some(intersect)
    }

    fn intersect_test(&self, ray: &Ray) -> bool {
        self.shape.intersect_test(ray)
    }

    fn material(&self) -> Option<&dyn Material> {
        self.material.as_ref().map(|m| m.as_ref()) // ugly?
    }

    fn area_light(&self) -> Option<&dyn AreaLight> {
        self.light.as_deref().map(|l| l as &dyn AreaLight)
    }

    fn light_arc_cloned(&self) -> Option<Arc<dyn Light>> {
        self.light.as_ref().map(|l| l.clone() as Arc<dyn Light>)
    }
}