use cgmath::{EuclideanSpace, InnerSpace};

use crate::{ComponentWiseExt, distance, Float, Normal3, Point2f, Vec3f, Point3f};
use crate::EFloat;
use crate::err_float::gamma;
use crate::geometry::{Ray, Transform};
use crate::geometry::bounds::Bounds3;
use crate::interaction::{DiffGeom, SurfaceHit};
use crate::interaction::SurfaceInteraction;
use crate::math::quadratic;
use crate::shapes::Shape;
use crate::sampling::uniform_sample_sphere;
use std::borrow::Borrow;

#[derive(Debug, PartialEq)]
pub struct Sphere<T: Borrow<Transform>=Transform> {
    object_to_world: T,
    world_to_object: T,
    reverse_orientation: bool,

    radius: Float,
    z_min: Float,
    z_max: Float,
    theta_min: Float,
    theta_max: Float,
    phi_max: Float
}

impl<T: Borrow<Transform>> Sphere<T> {
    pub fn new(
        object_to_world: T,
        world_to_object: T,
        reverse_orientation: bool,
        radius: Float,
        z_min: Float,
        z_max: Float,
        phi_max: Float
    ) -> Self {
        Self {
            object_to_world, world_to_object, reverse_orientation,
            radius,
            z_min: Float::min(z_min, z_max).clamp(-radius, radius),

            z_max: Float::max(z_min, z_max).clamp(-radius, radius),
            theta_min: Float::clamp(z_min / radius, -1.0, 1.0).acos(),
            theta_max: Float::clamp(z_max / radius, -1.0, 1.0).acos(),
            phi_max: phi_max.clamp(0.0, 360.0).to_radians()
        }
    }

    pub fn whole(
        object_to_world: T,
        world_to_object: T,
        radius: Float,
    ) -> Self {
        Self::new(object_to_world, world_to_object, false, radius, -radius, radius, 360.0)
    }
}

impl<T: Borrow<Transform> + Sync + Send> Shape for Sphere<T> {
    fn object_bound(&self) -> Bounds3<f32> {
        bounds3f!((-self.radius, -self.radius, self.z_min), (self.radius, self.radius, self.z_max))
    }

    fn object_to_world(&self) -> &Transform {
        self.object_to_world.borrow()
    }

    fn world_to_object(&self) -> &Transform {
        self.world_to_object.borrow()
    }

    fn reverse_orientation(&self) -> bool {
        self.reverse_orientation
    }

    fn area(&self) -> Float {
        self.phi_max * self.radius * (self.z_max - self.z_min)
    }

    #[allow(non_snake_case)]
    #[allow(clippy::many_single_char_names)]
    fn intersect(&self, ray: &Ray) -> Option<(Float, SurfaceInteraction)> {
        let (ray, (origin_err, dir_err)) = self.world_to_object().tf_exact_to_err(*ray);

        let ox = EFloat::with_err(ray.origin.x, origin_err.x);
        let oy = EFloat::with_err(ray.origin.y, origin_err.y);
        let oz = EFloat::with_err(ray.origin.z, origin_err.z);
        let dirx = EFloat::with_err(ray.dir.x, dir_err.x);
        let diry = EFloat::with_err(ray.dir.y, dir_err.y);
        let dirz = EFloat::with_err(ray.dir.z, dir_err.z);

        let a = dirx * dirx + diry * diry + dirz * dirz;
        let b = 2.0 * (dirx * ox + diry * oy + dirz * oz);
        let c = ox * ox + oy * oy + oz * oz - EFloat::new(self.radius) * EFloat::new(self.radius);

        let (t0, t1) = quadratic(a, b, c)?;

        if t0.upper_bound() > ray.t_max || t1.lower_bound() <= 0.0 {
            return None;
        }

        // find the closest valid intersection t value
        let mut t_shape_hit = t0;
        if t_shape_hit.lower_bound() <= 0.0 {
            t_shape_hit = t1;
            if t_shape_hit.upper_bound() > ray.t_max {
                return None
            }
        }

        let mut p_hit = ray.at(t_shape_hit.into());

        p_hit *= self.radius / distance(p_hit, point3f!(0, 0, 0));
        if p_hit.x == 0.0 && p_hit.y == 0.0 { p_hit.x = 1.0e-5 * self.radius }
        let mut phi = Float::atan2(p_hit.y, p_hit.x);
        if phi < 0.0 { phi += 2.0 * std::f32::consts::PI }


        // test against clipping parameters
        if (self.z_min > -self.radius && p_hit.z < self.z_min)
            || (self.z_max < self.radius && p_hit.z > self.z_max)
            || phi > self.phi_max
        {
            if t_shape_hit == t1 { return None; }
            if t1.upper_bound() > ray.t_max { return None; }

            t_shape_hit = t1;

            p_hit = ray.at(t_shape_hit.into());

            p_hit *= self.radius / distance(p_hit, point3f!(0, 0, 0));
            if p_hit.x == 0.0 && p_hit.y == 0.0 { p_hit.x = 1.0e-5 * self.radius }
            phi = Float::atan2(p_hit.y, p_hit.x);
            if phi < 0.0 { phi += 2.0 * std::f32::consts::PI }

            // If we still miss due to clipping
            if (self.z_min > -self.radius && p_hit.z < self.z_min)
                || (self.z_max < self.radius && p_hit.z > self.z_max)
                || phi > self.phi_max
            {
                return None;
            }
        }

        let u = phi / self.phi_max;
        let theta = Float::acos((p_hit.z / self.radius).clamp(-1.0, 1.0));
        let v = (theta - self.theta_min) / (self.theta_max - self.theta_min);

        let z_radius = (p_hit.x * p_hit.x + p_hit.y * p_hit.y).sqrt();
        let inv_z_radius = 1.0 / z_radius;
        let cos_phi = p_hit.x * inv_z_radius;
        let sin_phi = p_hit.y * inv_z_radius;

        let dpdu = vec3f!(-self.phi_max * p_hit.y, self.phi_max * p_hit.x, 0.0);
        let dpdv = (self.theta_max - self.theta_min) *
            vec3f!(p_hit.z * cos_phi, p_hit.z * sin_phi, -self.radius * theta.sin());

        let d2pduu = (-self.phi_max * self.phi_max) * vec3f!(p_hit.x, p_hit.y, 0.0);
        let d2pduv = (self.theta_max - self.theta_min) * p_hit.z * self.phi_max * vec3f!(-sin_phi, cos_phi, 0.0);
        let d2pdvv = -(self.theta_max - self.theta_min) * (self.theta_max - self.theta_min) *
            vec3f!(p_hit.x, p_hit.y, p_hit.z);

        let E = dpdu.dot(dpdu);
        let F = dpdu.dot(dpdv);
        let G = dpdv.dot(dpdv);

        let mut N = dpdu.cross(dpdv).normalize();

        let e = N.dot(d2pduu);
        let f = N.dot(d2pduv);
        let g = N.dot(d2pdvv);

        let invEGF2 = 1.0 / (E * G - F * F);

        let dndu = Normal3((f * F - e * G) * invEGF2 * dpdu + (e * F - f * E) * invEGF2 * dpdv);

        let dndv = Normal3((g * F - f * G) * invEGF2 * dpdu + (f * F - g * E) * invEGF2 * dpdv);

        let p_err: Vec3f = gamma(5) * p_hit.to_vec().abs();

        // FIXME
        if self.reverse_orientation() {
            N *= -1.0;
        }

        let interact = SurfaceInteraction::new(
            p_hit,
            p_err,
            ray.time,
            Point2f::new(u, v),
            -ray.dir,
            Normal3(N),
            DiffGeom { dpdu, dpdv, dndu, dndv }
        );

        let world_intersect = self.object_to_world().borrow().transform(interact);

        Some((t_shape_hit.into(), world_intersect))
    }

    fn sample(&self, u: Point2f) -> SurfaceHit {
        let mut p_obj = Point3f::new(0.0, 0.0, 0.0) + self.radius * uniform_sample_sphere(u);
        let mut n = Normal3(self.object_to_world.borrow().transform(Normal3(p_obj.to_vec())).normalize());
        if self.reverse_orientation {
            n *= -1.0;
        }
        // re-project p_obj to sphere surface
        p_obj *= self.radius / distance(p_obj, Point3f::new(0.0, 0.0, 0.0));
        let p_obj_err = gamma(5) * p_obj.to_vec().abs();
        let (p, p_err) = self.object_to_world.borrow().tf_err_to_err(p_obj, p_obj_err);
        SurfaceHit {
            p,
            p_err,
            time: 0.0,
            n
        }
    }

//    fn intersect_test(&self, ray: &Ray) -> bool {
//        unimplemented!()
//    }
}

#[cfg(test)]
mod tests {
    use cgmath::assert_abs_diff_eq;
    use rand::SeedableRng;

    use crate::Point3f;
    use crate::sampling::rejection_sample_shere;

    use super::*;

    fn shoot_ray(from: impl Into<Point3f> + Copy, to: impl Into<Point3f> + Copy) -> Ray {
        let dir = to.into() - from.into();
        Ray::new(from.into(), dir)
    }

    #[test]
    fn test_whole_sphere_intersect() {
        let o2w = Transform::translate((0.0, 0.0, 0.0).into());
        let w2o = o2w.inverse();

        let radius = 1.0;
        let sphere = Sphere::whole(&o2w, &w2o, radius);

        let orig = Point3f::new(3.0, 3.0, 3.0);
        let mut rng = rand::rngs::SmallRng::from_seed([4; 16]);
        for _ in 0..100 {
            let point_in_sphere = rejection_sample_shere(&mut rng, radius);
            let ray = shoot_ray(orig, point_in_sphere);

            let isect = sphere.intersect(&ray);
            assert!(isect.is_some(), "{:?} {:?}", ray, point_in_sphere);
            let err = isect.unwrap().1.hit.p_err;
            assert_abs_diff_eq!(err, Vec3f::new(0.0, 0.0, 0.0), epsilon = 0.0001);
        }

        let orig = Point3f::new(1.0, 0.0, -2.0);

        let edge_point = Point3f::new(1.0, 0.0, 0.0);
        let ray = shoot_ray(orig, edge_point);
        let isect = sphere.intersect(&ray);
        assert!(isect.is_some());
        let err = isect.unwrap().1.hit.p_err;
        assert_abs_diff_eq!(err, Vec3f::new(0.0, 0.0, 0.0), epsilon = 0.0001);


        let close_miss = Point3f::new(1.0 + 0.0001, 0.0, 0.0);
        let ray = shoot_ray(orig, close_miss);
        assert!(sphere.intersect(&ray).is_none());
    }
}