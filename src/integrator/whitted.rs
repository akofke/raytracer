use bumpalo::Bump;

use crate::{abs_dot, RayDifferential};
use crate::integrator::IntegratorRadiance;
use crate::material::TransportMode;
use crate::reflection::BxDFType;
use crate::sampler::Sampler;
use crate::scene::Scene;
use crate::spectrum::Spectrum;

pub struct WhittedIntegrator {
    pub max_depth: u16,
}

impl IntegratorRadiance for WhittedIntegrator {
    fn preprocess(&mut self, _scene: &Scene, _sampler: &mut dyn Sampler) {
        // unimplemented!()
    }

    fn incident_radiance(&self, ray: &mut RayDifferential, scene: &Scene, sampler: &mut dyn Sampler, arena: &Bump, depth: u16) -> Spectrum {
        let mut radiance: Spectrum = Spectrum::uniform(0.0);

        match scene.intersect(&mut ray.ray) {
            None => {
                scene.environment_emitted_radiance(ray)
            },

            Some(mut intersect) => {
                let n = intersect.shading_n;
                let wo = intersect.wo;

                let bsdf = intersect.compute_scattering_functions(
                    ray,
                    arena,
                    false,
                    TransportMode::Radiance
                );

                if let Some(bsdf) = bsdf {

                    for light in scene.lights.iter() {
                        let li_sample = light.sample_incident_radiance(
                            &intersect.hit,
                            sampler.get_2d(),
                        );

                        if li_sample.radiance.is_black() || li_sample.pdf == 0.0 {
                            continue;
                        }

                        let f = bsdf.f(wo, li_sample.wi, BxDFType::all());

                        if !f.is_black() && li_sample.vis.unoccluded(scene) {
                            radiance += f * li_sample.radiance * abs_dot(li_sample.wi, n.0) / li_sample.pdf;
                        }
                    }

                    if depth + 1 < self.max_depth {
                        radiance += self.specular_reflect(ray, &intersect, &bsdf, scene, sampler, arena, depth);
                        radiance += self.specular_transmit(ray, &intersect, &bsdf, scene, sampler, arena, depth);
                    }
                } else {
                    unimplemented!()
                }

                radiance
            }

        }
    }
}



