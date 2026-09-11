// [WFGY] Zone: SAFE | λ: 0.25 | Fallbacks: 0/None | Action: Dynamic Inspectable Property Reflection System for AORUI Studio Palettes
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum PropertyKind {
    Float { min: f32, max: f32, step: f32 },
    ColorRgba,
    Vec2,
    Vec3 { min: f32, max: f32 },
    Bool,
    Choice { options: Vec<String> },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum PropertyValue {
    Float(f32),
    ColorRgba([f32; 4]),
    Vec2([f32; 2]),
    Vec3([f32; 3]),
    Bool(bool),
    Choice(String),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PropertyDesc {
    pub id: String,
    pub name: String,
    pub category: String,
    pub kind: PropertyKind,
    pub value: PropertyValue,
}

pub trait Inspectable {
    fn inspect_properties(&self) -> Vec<PropertyDesc>;
    fn set_property(&mut self, id: &str, value: PropertyValue) -> Result<(), String>;
}

// Implémentation pour MaterialUniform
impl Inspectable for crate::scene::node::MaterialUniform {
    fn inspect_properties(&self) -> Vec<PropertyDesc> {
        vec![
            PropertyDesc {
                id: "roughness".into(),
                name: "Rugosité (Roughness)".into(),
                category: "PBR Surface".into(),
                kind: PropertyKind::Float { min: 0.0, max: 1.0, step: 0.01 },
                value: PropertyValue::Float(self.roughness),
            },
            PropertyDesc {
                id: "metallic".into(),
                name: "Métallique (Metallic)".into(),
                category: "PBR Surface".into(),
                kind: PropertyKind::Float { min: 0.0, max: 1.0, step: 0.01 },
                value: PropertyValue::Float(self.metallic),
            },
            PropertyDesc {
                id: "ior".into(),
                name: "Indice Réfraction (IOR)".into(),
                category: "Optique & Verre".into(),
                kind: PropertyKind::Float { min: 1.0, max: 3.0, step: 0.02 },
                value: PropertyValue::Float(self.ior),
            },
            PropertyDesc {
                id: "transmission".into(),
                name: "Transparence (Transmission)".into(),
                category: "Optique & Verre".into(),
                kind: PropertyKind::Float { min: 0.0, max: 1.0, step: 0.01 },
                value: PropertyValue::Float(self.transmission),
            },
            PropertyDesc {
                id: "clearcoat".into(),
                name: "Vernis (Clearcoat)".into(),
                category: "Vernis & Finition".into(),
                kind: PropertyKind::Float { min: 0.0, max: 1.0, step: 0.01 },
                value: PropertyValue::Float(self.clearcoat),
            },
            PropertyDesc {
                id: "clearcoat_roughness".into(),
                name: "Rugosité Vernis".into(),
                category: "Vernis & Finition".into(),
                kind: PropertyKind::Float { min: 0.0, max: 1.0, step: 0.01 },
                value: PropertyValue::Float(self.clearcoat_roughness),
            },
            PropertyDesc {
                id: "subsurface".into(),
                name: "Diffusion Sous-Surfacique (SSS)".into(),
                category: "Matériaux Avancés".into(),
                kind: PropertyKind::Float { min: 0.0, max: 1.0, step: 0.01 },
                value: PropertyValue::Float(self.subsurface),
            },
            PropertyDesc {
                id: "emission_color".into(),
                name: "Couleur Émission HDR".into(),
                category: "Émission".into(),
                kind: PropertyKind::ColorRgba,
                value: PropertyValue::ColorRgba(self.emission_color),
            },
        ]
    }

    fn set_property(&mut self, id: &str, value: PropertyValue) -> Result<(), String> {
        match (id, value) {
            ("roughness", PropertyValue::Float(v)) => self.roughness = v.clamp(0.0, 1.0),
            ("metallic", PropertyValue::Float(v)) => self.metallic = v.clamp(0.0, 1.0),
            ("ior", PropertyValue::Float(v)) => self.ior = v.max(1.0),
            ("transmission", PropertyValue::Float(v)) => self.transmission = v.clamp(0.0, 1.0),
            ("clearcoat", PropertyValue::Float(v)) => self.clearcoat = v.clamp(0.0, 1.0),
            ("clearcoat_roughness", PropertyValue::Float(v)) => self.clearcoat_roughness = v.clamp(0.0, 1.0),
            ("subsurface", PropertyValue::Float(v)) => self.subsurface = v.clamp(0.0, 1.0),
            ("emission_color", PropertyValue::ColorRgba(c)) => self.emission_color = c,
            _ => return Err(format!("Propriété inconnue ou type incompatible: {}", id)),
        }
        Ok(())
    }
}

// Implémentation pour Transform
impl Inspectable for crate::scene::node::Transform {
    fn inspect_properties(&self) -> Vec<PropertyDesc> {
        vec![
            PropertyDesc {
                id: "translation".into(),
                name: "Position (X, Y, Z)".into(),
                category: "Transform".into(),
                kind: PropertyKind::Vec3 { min: -1000.0, max: 1000.0 },
                value: PropertyValue::Vec3([self.translation.x, self.translation.y, self.translation.z]),
            },
            PropertyDesc {
                id: "rotation".into(),
                name: "Rotation Degrés (X, Y, Z)".into(),
                category: "Transform".into(),
                kind: PropertyKind::Vec3 { min: -360.0, max: 360.0 },
                value: PropertyValue::Vec3([
                    self.rotation.x.to_degrees(),
                    self.rotation.y.to_degrees(),
                    self.rotation.z.to_degrees(),
                ]),
            },
            PropertyDesc {
                id: "scale".into(),
                name: "Échelle (X, Y, Z)".into(),
                category: "Transform".into(),
                kind: PropertyKind::Vec3 { min: 0.001, max: 100.0 },
                value: PropertyValue::Vec3([self.scale.x, self.scale.y, self.scale.z]),
            },
        ]
    }

    fn set_property(&mut self, id: &str, value: PropertyValue) -> Result<(), String> {
        match (id, value) {
            ("translation", PropertyValue::Vec3(v)) => self.translation = glam::Vec3::from_array(v),
            ("rotation", PropertyValue::Vec3(v)) => {
                self.rotation = glam::Vec3::new(
                    v[0].to_radians(),
                    v[1].to_radians(),
                    v[2].to_radians(),
                );
            }
            ("scale", PropertyValue::Vec3(v)) => self.scale = glam::Vec3::from_array(v),
            _ => return Err(format!("Propriété Transform inconnue: {}", id)),
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::node::MaterialUniform;

    #[test]
    fn test_material_inspectable_reflection_and_set() {
        let mut mat = MaterialUniform::default();
        let props = mat.inspect_properties();
        assert!(!props.is_empty());
        assert!(props.iter().any(|p| p.id == "clearcoat"));
        assert!(props.iter().any(|p| p.id == "subsurface"));

        mat.set_property("clearcoat", PropertyValue::Float(0.85)).unwrap();
        assert_eq!(mat.clearcoat, 0.85);

        mat.set_property("subsurface", PropertyValue::Float(0.4)).unwrap();
        assert_eq!(mat.subsurface, 0.4);
    }
}
