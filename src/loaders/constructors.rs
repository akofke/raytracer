use crate::loaders::{ParamSet, ParamError};
use crate::shapes::sphere::Sphere;
use crate::{Transform, Float, Point3f};
use crate::material::matte::MatteMaterial;
use crate::shapes::triangle::TriangleMesh;
use crate::light::diffuse::DiffuseAreaLightBuilder;
use crate::spectrum::Spectrum;
use crate::texture::checkerboard::{Checkerboard2DTexture};
use crate::texture::mapping::{TexCoordsMap2D, UVMapping};
use std::sync::Arc;
use crate::texture::Texture;
use crate::light::distant::DistantLight;
use crate::light::point::PointLight;

type ParamResult<T> = Result<T, ConstructError>;

#[derive(Debug)]
pub enum ConstructError {
    ParamError(ParamError),
    ValueError(String),
}

impl From<ParamError> for ConstructError {
    fn from(e: ParamError) -> Self {
        Self::ParamError(e)
    }
}

pub fn make_sphere(mut params: ParamSet) -> ParamResult<Sphere<Transform>> {
    let radius = params.get_one("radius").unwrap_or(1.0);
    let zmin = params.get_one("zmin").unwrap_or(-radius);
    let zmax = params.get_one("zmax").unwrap_or(radius);
    let phimax = params.get_one("phimax").unwrap_or(360.0);
    let o2w = params.current_transform()?;
    let w2o = o2w.inverse();
    let rev = params.reverse_orientation()?;
    Ok(Sphere::new(
        o2w,
        w2o,
        rev,
        radius,
        zmin,
        zmax,
        phimax
    ))
}

pub fn make_triangle_mesh(mut params: ParamSet) -> ParamResult<TriangleMesh> {
    let tf = params.current_transform()?;
    let indices: Vec<i32> = params.get_one("indices")?;
    let indices = indices.into_iter().map(|i| i as u32).collect();
    let vertices = params.get_one("P")?;
    let normals = params.get_one("N").ok();
    let tangents = params.get_one("S").ok();
    // TODO: handle float array
    let tex_coords = params.get_one("uv").or_else(|_| params.get_one("st")).ok();
    let reverse_orientation = params.reverse_orientation()?;

    let mesh = TriangleMesh::new(
        tf,
        indices,
        vertices,
        normals,
        tangents,
        tex_coords,
        reverse_orientation
    );
    Ok(mesh)
}

pub fn make_matte(mut params: ParamSet) -> ParamResult<MatteMaterial> {
    let diffuse = params.get_texture_or_const("Kd")?;
    Ok(MatteMaterial::new(diffuse))
}

pub fn make_diffuse_area_light(mut params: ParamSet) -> ParamResult<DiffuseAreaLightBuilder> {
    let emit = params.get_one("L").unwrap_or(Spectrum::new(1.0));
    let _two_sided = params.get_one("twosided").unwrap_or(false);
    let samples = params.get_one("samples").unwrap_or(1) as usize;
    Ok(DiffuseAreaLightBuilder { emit, n_samples: samples })
}

fn make_tex_coords_map_2d(params: &mut ParamSet) -> Result<Arc<dyn TexCoordsMap2D>, ConstructError> {
    let map_type = params.get_one("mapping").unwrap_or_else(|_| "uv".to_string());
    match map_type.as_ref() {
        "uv" => {
            let uscale = params.get_one("uscale").unwrap_or(1.0);
            let vscale = params.get_one("vscale").unwrap_or(1.0);
            let udelta = params.get_one("udelta").unwrap_or(0.0);
            let vdelta = params.get_one("vdelta").unwrap_or(0.0);
            let map = UVMapping::new(uscale, vscale, udelta, vdelta);
            Ok(Arc::new(map))
        }
        _ => Err(ConstructError::ValueError(format!("Unknown mapping type {}", map_type)))
    }

}

pub fn make_checkerboard_float(mut params: ParamSet) -> ParamResult<Arc<dyn Texture<Output=Float>>> {
    let mapping = make_tex_coords_map_2d(&mut params)?;
    let tex1 = params.get_texture_or_const::<Float>("tex1")?;
    let tex2 = params.get_texture_or_const::<Float>("tex2")?;

    let tex = Arc::new(Checkerboard2DTexture::new(
        tex1,
        tex2,
        mapping
    ));
    Ok(tex)
}

pub fn make_checkerboard_spect(mut params: ParamSet) -> ParamResult<Arc<dyn Texture<Output=Spectrum>>> {
    let mapping = make_tex_coords_map_2d(&mut params)?;
    let tex1 = params.get_texture_or_const::<Spectrum>("tex1")?;
    let tex2 = params.get_texture_or_const::<Spectrum>("tex2")?;

    let tex = Arc::new(Checkerboard2DTexture::new(
        tex1,
        tex2,
        mapping
    ));
    Ok(tex)
}

pub fn make_distant_light(mut params: ParamSet) -> ParamResult<DistantLight> {
    let radiance = params.get_one("L").unwrap_or(Spectrum::new(1.0));
    let scale = params.get_one("scale").unwrap_or(Spectrum::new(1.0));
    let radiance = radiance * scale;
    let from = params.get_one("from").unwrap_or(Point3f::new(0.0, 0.0, 0.0));
    let to = params.get_one("to").unwrap_or(Point3f::new(0.0, 0.0, 1.0));
    Ok(DistantLight::from_to(from, to, radiance))
}

pub fn make_point_light(mut params: ParamSet) -> ParamResult<PointLight> {
    let intensity = params.get_one("I").unwrap_or(Spectrum::new(1.0));
    let scale = params.get_one("scale").unwrap_or(Spectrum::new(1.0));
    let intensity = intensity * scale;
    let from = params.get_one("from").unwrap_or(Point3f::new(0.0, 0.0, 0.0));
    let light_to_world = Transform::translate(from - Point3f::new(0.0, 0.0, 0.0));
    Ok(PointLight::new(light_to_world, intensity))
}