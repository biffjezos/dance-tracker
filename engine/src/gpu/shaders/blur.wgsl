// src/gpu/shaders/blur.wgsl

struct Pixels {
    data: array<f32>,
};

struct Params {
    width: u32,
    height: u32,
    radius: u32,
    _padding: u32,
};

@group(0)
@binding(0)
var<storage, read> input: Pixels;

@group(0)
@binding(1)
var<storage, read_write> output: Pixels;

@group(0)
@binding(2)
var<uniform> params: Params;


@compute
@workgroup_size(16,16)

fn main(
    @builtin(global_invocation_id) id: vec3<u32>
) {
    if (id.x >= params.width || id.y >= params.height) {
        return;
    }

    let width = params.width;
    let height = params.height;
    let radius = params.radius;

    var sum_r = 0.0;
    var sum_g = 0.0;
    var sum_b = 0.0;
    var sum_a = 0.0;

    var count = 0.0;

    for (var y = -i32(radius); y <= i32(radius); y++) {
        for (var x = -i32(radius); x <= i32(radius); x++) {

            let px = clamp( i32(id.x) + x, 0, i32(width) - 1 );
            let py = clamp( i32(id.y) + y, 0, i32(height) - 1  );
            let index = (u32(py) * width + u32(px)) * 4u;
            sum_r += input.data[index];
            sum_g += input.data[index + 1u];
            sum_b += input.data[index + 2u];
            sum_a += input.data[index + 3u];
            count += 1.0;
        }
    }

    let output_index = (id.y * width + id.x) * 4u;
    output.data[output_index] = sum_r / count;
    output.data[output_index + 1u] = sum_g / count;
    output.data[output_index + 2u] = sum_b / count;
    output.data[output_index + 3u] = sum_a / count;
}