use crate::camera::Camera;
use crate::film::Film;
use crate::filter::BoxFilter;
use crate::sampler::Sampler;
use crate::scene::Scene;
use crate::spectrum::{RGBSpectrum, Spectrum};
use crate::{Bounds2i, Float, RayDifferential, SurfaceInteraction, Normal3, Differential};
use bumpalo::Bump;
use rayon::prelude::*;
use crate::reflection::bsdf::Bsdf;
use crate::reflection::BxDFType;
use cgmath::InnerSpace;

pub mod whitted;

pub trait Integrator {
    fn render(&mut self, scene: &Scene, film: &Film<BoxFilter>);
}

pub struct SamplerIntegrator<R: IntegratorRadiance> {
    pub camera: Box<dyn Camera>,
    pub sampler: Box<dyn Sampler>,
    pub radiance: R,
}

pub trait IntegratorRadiance: Sync + Send {
    fn preprocess(&mut self, scene: &Scene, sampler: &dyn Sampler);

    fn incident_radiance(
        &self,
        ray: &mut RayDifferential,
        scene: &Scene,
        sampler: &mut dyn Sampler,
        arena: &Bump,
        depth: u16,
    ) -> Spectrum;

    #[allow(non_snake_case)]
    fn specular_reflect(
        &self,
        ray: &mut RayDifferential,
        intersect: &SurfaceInteraction,
        bsdf: &Bsdf,
        scene: &Scene,
        sampler: &mut dyn Sampler,
        arena: &Bump,
        depth: u16,
    ) -> Spectrum {
        let wo = intersect.wo;
        let bxdf_type = BxDFType::REFLECTION | BxDFType::SPECULAR;

        if let Some(scatter) = bsdf.sample_f(wo, sampler.get_2d(), bxdf_type) {
            let diff = ray.diff.map(|diff| {
                let tex_diff = intersect.tex_diffs.unwrap(); // TODO make this cleaner. It must have them at this point
                let rx_origin = intersect.hit.p + tex_diff.dpdx;
                let ry_origin = intersect.hit.p + tex_diff.dpdy;

                let shading = intersect.shading_geom;
                let dndx = shading.dndu * tex_diff.dudx + shading.dndv * tex_diff.dvdx;
                let dndy = shading.dndu * tex_diff.dudy + shading.dndv * tex_diff.dvdy;

                let dwo_dx = -diff.rx_dir - wo;
                let dwo_dy = -diff.ry_dir - wo;

                let dDN_dx = dwo_dx.dot(intersect.shading_n.0) + wo.dot(dndx.0);
                let dDN_dy = dwo_dy.dot(intersect.shading_n.0) + wo.dot(dndy.0);

                let rx_dir = (scatter.wi - dwo_dx) + 2.0 * wo.dot(intersect.shading_n.0) * dndx.0 + dDN_dx * intersect.shading_n.0;
                let ry_dir = (scatter.wi - dwo_dy) + 2.0 * wo.dot(intersect.shading_n.0) * dndy.0 + dDN_dy * intersect.shading_n.0;

                Differential {
                    rx_origin,
                    rx_dir,
                    ry_origin,
                    ry_dir
                }
            });

            let mut ray_diff = intersect.hit.spawn_ray_with_dfferentials(scatter.wi, diff);
            let li = self.incident_radiance(
                &mut ray_diff,
                scene,
                sampler,
                arena,
                depth + 1
            );
            return scatter.f * li * scatter.wi.dot(intersect.shading_n.0).abs() / scatter.pdf;
        } else {
            return Spectrum::new(0.0);
        }
    }
}

impl<R: IntegratorRadiance> Integrator for SamplerIntegrator<R> {
    fn render(&mut self, scene: &Scene, film: &Film<BoxFilter>) {
        // preprocess
        let sample_bounds = film.sample_bounds();
        sample_bounds.iter_tiles(16).par_bridge().for_each(|tile| {
            let mut arena = Bump::new();

            let tile_id = Self::tile_id(tile, sample_bounds);
            let mut tile_sampler = self.sampler.clone_with_seed(tile_id);

            let mut film_tile = film.get_film_tile(tile);

            for pixel in tile.iter_points() {
                tile_sampler.start_pixel(pixel.into());

                while tile_sampler.start_next_sample() {
                    let camera_sample = tile_sampler.get_camera_sample(pixel.into());

                    let (ray_weight, mut ray_differential) =
                        self.camera.generate_ray_differential(camera_sample);

                    ray_differential.scale_differentials(
                        1.0 / (tile_sampler.samples_per_pixel() as Float).sqrt(),
                    );

                    let mut radiance = Spectrum::<RGBSpectrum>::new(0.0);

                    if ray_weight > 0.0 {
                        radiance = self.radiance.incident_radiance(
                            &mut ray_differential,
                            scene,
                            tile_sampler.as_mut(),
                            &arena,
                            0,
                        );

                        check_radiance(&radiance, pixel);
                    }

                    film.add_sample_to_tile(
                        &mut film_tile,
                        camera_sample.p_film,
                        radiance,
                        ray_weight,
                    );

                    arena.reset();
                }
            }

            film.merge_film_tile(film_tile);
        });
    }
}

impl<R: IntegratorRadiance> SamplerIntegrator<R> {
    fn tile_id(tile: Bounds2i, sample_bounds: Bounds2i) -> u64 {
        let n_cols = sample_bounds.max.x;
        (tile.min.y * n_cols + tile.min.x) as u64
    }

    pub fn render_with_pool(&mut self, scene: &Scene, film: &Film<BoxFilter>, pool: &rayon::ThreadPool) {
        pool.install(|| self.render(scene, film))
    }

}

fn check_radiance(l: &Spectrum, pixel: (i32, i32)) {
    assert!(!l.has_nans(), "NaN radiance value for pixel {:?}", pixel);
}
