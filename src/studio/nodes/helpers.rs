// [WFGY] Zone: SAFE | λ: 0.2 | Fallbacks: 0/None | Action: WGSL procedural helper library injected on demand by the graph compiler
#![allow(dead_code)]

/// Ensemble des helpers WGSL requis par une compilation donnée
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct HelperFlags {
    pub perlin: bool,
    pub simplex: bool,
    pub voronoi: bool,
    pub checker: bool,
    pub gradient: bool,
    pub remap: bool,
    pub hsv: bool,
    pub desaturate: bool,
    pub fresnel: bool,
    pub parallax: bool,
    pub distance_edge: bool,
    pub wave: bool,
}

impl HelperFlags {
    /// Liste triée des noms de helpers actifs
    pub fn active(&self) -> Vec<&'static str> {
        let mut v = Vec::new();
        if self.perlin {
            v.push("perlin");
        }
        if self.simplex {
            v.push("simplex");
        }
        if self.voronoi {
            v.push("voronoi");
        }
        if self.checker {
            v.push("checker");
        }
        if self.gradient {
            v.push("gradient");
        }
        if self.remap {
            v.push("remap");
        }
        if self.hsv {
            v.push("hsv");
        }
        if self.desaturate {
            v.push("desaturate");
        }
        if self.fresnel {
            v.push("fresnel");
        }
        if self.parallax {
            v.push("parallax");
        }
        if self.distance_edge {
            v.push("distance_edge");
        }
        if self.wave {
            v.push("wave");
        }
        v
    }
}

const HASH: &str = r#"
fn aor_hash2(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(127.1, 311.7))) * 43758.5453123);
}
"#;

const PERLIN: &str = r#"
fn aor_perlin2(p: vec2<f32>) -> f32 {
    let pi = floor(p);
    let pf = fract(p);
    let u = pf * pf * (3.0 - 2.0 * pf);
    let a = aor_hash2(pi + vec2<f32>(0.0, 0.0));
    let b = aor_hash2(pi + vec2<f32>(1.0, 0.0));
    let c = aor_hash2(pi + vec2<f32>(0.0, 1.0));
    let d = aor_hash2(pi + vec2<f32>(1.0, 1.0));
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}
"#;

const SIMPLEX: &str = r#"
fn aor_simplex2(p: vec2<f32>) -> f32 {
    let s = (p.x + p.y) * 0.3660254037844386;
    let i = floor(p + vec2<f32>(s));
    let t = (i.x + i.y) * 0.21132486540518713;
    let x0 = p - i + vec2<f32>(t);
    var i1 = vec2<f32>(1.0, 0.0);
    if (x0.x <= x0.y) { i1 = vec2<f32>(0.0, 1.0); }
    let n = aor_perlin2(i)
          + 0.5 * aor_perlin2(i + i1)
          + 0.25 * aor_perlin2(i + vec2<f32>(1.0, 1.0));
    return n * 0.5 + 0.5;
}
"#;

const VORONOI: &str = r#"
fn aor_voronoi2(p: vec2<f32>) -> f32 {
    let n = floor(p);
    let f = fract(p);
    var min_dist = 8.0;
    for (var j: i32 = -1; j <= 1; j = j + 1) {
        for (var i: i32 = -1; i <= 1; i = i + 1) {
            let g = vec2<f32>(f32(i), f32(j));
            let o = vec2<f32>(aor_hash2(n + g), aor_hash2(n + g + vec2<f32>(5.2, 1.3)));
            let r = g + o - f;
            let d = dot(r, r);
            min_dist = min(min_dist, d);
        }
    }
    return sqrt(min_dist);
}
"#;

const CHECKER: &str = r#"
fn aor_checker(p: vec2<f32>, scale: f32) -> f32 {
    let q = floor(p * scale);
    return (q.x + q.y) % 2.0;
}
"#;

const GRADIENT: &str = r#"
fn aor_gradient(t: f32) -> f32 {
    return clamp(t, 0.0, 1.0);
}
"#;

const REMAP: &str = r#"
fn aor_remap(v: f32, in_min: f32, in_max: f32, out_min: f32, out_max: f32) -> f32 {
    let denom = max(in_max - in_min, 1e-6);
    return out_min + (v - in_min) / denom * (out_max - out_min);
}
"#;

const HSV: &str = r#"
fn aor_hsv_to_rgb(c: vec4<f32>) -> vec4<f32> {
    let h = c.x * 6.0;
    let s = c.y;
    let v = c.z;
    let i = floor(h);
    let f = h - i;
    let p = v * (1.0 - s);
    let q = v * (1.0 - s * f);
    let t = v * (1.0 - s * (1.0 - f));
    let m = i % 6.0;
    var rgb = vec3<f32>(v, t, p);
    if (m < 1.0) {
        rgb = vec3<f32>(v, t, p);
    } else if (m < 2.0) {
        rgb = vec3<f32>(q, v, p);
    } else if (m < 3.0) {
        rgb = vec3<f32>(p, v, t);
    } else if (m < 4.0) {
        rgb = vec3<f32>(p, q, v);
    } else if (m < 5.0) {
        rgb = vec3<f32>(t, p, v);
    } else {
        rgb = vec3<f32>(v, p, q);
    }
    return vec4<f32>(rgb, c.w);
}
"#;

const DESATURATE: &str = r#"
fn aor_desaturate(c: vec4<f32>, amount: f32) -> vec4<f32> {
    let l = dot(c.rgb, vec3<f32>(0.299, 0.587, 0.114));
    return vec4<f32>(mix(c.rgb, vec3<f32>(l), amount), c.a);
}
"#;

const FRESNEL: &str = r#"
fn aor_fresnel(normal: vec3<f32>, view_dir: vec3<f32>, power: f32, bias: f32) -> f32 {
    let facing = clamp(dot(normalize(normal), normalize(view_dir)), 0.0, 1.0);
    let f = pow(1.0 - facing, power);
    return bias + (1.0 - bias) * f;
}
"#;

const PARALLAX: &str = r#"
fn aor_parallax_occlusion(uv: vec2<f32>, height: f32, scale: f32) -> vec2<f32> {
    return uv + vec2<f32>(height * scale, 0.0);
}
"#;

const DISTANCE_EDGE: &str = r#"
fn aor_distance_to_edge(uv: vec2<f32>, size: f32) -> f32 {
    let d = min(min(uv.x, 1.0 - uv.x), min(uv.y, 1.0 - uv.y));
    return clamp(d * size, 0.0, 1.0);
}
"#;

const WAVE: &str = r#"
fn aor_wave_displacement(pos: vec3<f32>, time: f32, amplitude: f32, frequency: f32, speed: f32) -> vec3<f32> {
    let w = sin(pos.x * frequency + time * speed) * amplitude;
    return vec3<f32>(pos.x, pos.y + w, pos.z);
}
"#;

/// Construit la section de helpers WGSL correspondant aux drapeaux actifs
pub fn build_helpers(flags: &HelperFlags) -> String {
    let mut out = String::new();
    // `aor_hash2` est requis par Perlin, Simplex et Voronoi
    if flags.perlin || flags.simplex || flags.voronoi {
        out.push_str(HASH);
    }
    if flags.perlin || flags.simplex {
        out.push_str(PERLIN);
    }
    if flags.simplex {
        out.push_str(SIMPLEX);
    }
    if flags.voronoi {
        out.push_str(VORONOI);
    }
    if flags.checker {
        out.push_str(CHECKER);
    }
    if flags.gradient {
        out.push_str(GRADIENT);
    }
    if flags.remap {
        out.push_str(REMAP);
    }
    if flags.hsv {
        out.push_str(HSV);
    }
    if flags.desaturate {
        out.push_str(DESATURATE);
    }
    if flags.fresnel {
        out.push_str(FRESNEL);
    }
    if flags.parallax {
        out.push_str(PARALLAX);
    }
    if flags.distance_edge {
        out.push_str(DISTANCE_EDGE);
    }
    if flags.wave {
        out.push_str(WAVE);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_flags_no_helpers() {
        assert!(build_helpers(&HelperFlags::default()).is_empty());
    }

    #[test]
    fn test_perlin_pulls_hash() {
        let flags = HelperFlags {
            perlin: true,
            ..Default::default()
        };
        let src = build_helpers(&flags);
        assert!(src.contains("aor_hash2"));
        assert!(src.contains("aor_perlin2"));
    }

    #[test]
    fn test_active_list() {
        let flags = HelperFlags {
            voronoi: true,
            hsv: true,
            ..Default::default()
        };
        assert_eq!(flags.active(), vec!["voronoi", "hsv"]);
    }
}
