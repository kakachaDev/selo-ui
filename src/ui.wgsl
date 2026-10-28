struct Screen {
    size: vec4<f32>,
};

@group(0) @binding(0) var<uniform> screen: Screen;
@group(0) @binding(1) var atlas: texture_2d<f32>;
@group(0) @binding(2) var atlas_sampler: sampler;
@group(1) @binding(0) var image: texture_2d<f32>;
@group(1) @binding(1) var image_sampler: sampler;

struct VIn {
    @location(0) pos: vec2<f32>,
    @location(1) local: vec2<f32>,
    @location(2) half: vec2<f32>,
    @location(3) params: vec3<f32>,
    @location(4) color: vec4<f32>,
};

struct VOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) local: vec2<f32>,
    @location(1) half: vec2<f32>,
    @location(2) params: vec3<f32>,
    @location(3) color: vec4<f32>,
};

@vertex
fn vs_main(v: VIn) -> VOut {
    var o: VOut;
    let ndc = v.pos / screen.size.xy * 2.0 - vec2<f32>(1.0, 1.0);
    o.pos = vec4<f32>(ndc.x, -ndc.y, 0.0, 1.0);
    o.local = v.local;
    o.half = v.half;
    o.params = v.params;
    o.color = v.color;
    return o;
}

fn round_box(p: vec2<f32>, b: vec2<f32>, r: f32) -> f32 {
    let q = abs(p) - b + vec2<f32>(r, r);
    return length(max(q, vec2<f32>(0.0, 0.0))) + min(max(q.x, q.y), 0.0) - r;
}

@fragment
fn fs_main(i: VOut) -> @location(0) vec4<f32> {
    let glyph = textureSample(atlas, atlas_sampler, i.local).r;
    let img = textureSample(image, image_sampler, i.local);
    let px = screen.size.z;
    let d = round_box(i.local, i.half, i.params.x) * px;
    let fill = clamp(0.5 - d, 0.0, 1.0);
    let inner = clamp(0.5 - (d + i.params.z * px), 0.0, 1.0);
    var a = glyph;
    if (i.params.y > 1.5) {
        a = fill * (1.0 - inner);
    } else if (i.params.y > 0.5) {
        a = fill;
    }
    var rgb = i.color.rgb;
    if (screen.size.w > 0.5) {
        rgb = pow(rgb, vec3<f32>(2.2, 2.2, 2.2));
    }
    if (i.params.y > 2.5) {
        return vec4<f32>(rgb * img.rgb, i.color.a * img.a);
    }
    return vec4<f32>(rgb, i.color.a * a);
}
