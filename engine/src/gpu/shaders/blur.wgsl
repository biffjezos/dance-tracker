struct Pixels {
    data: array<f32>,
};

@group(0)
@binding(0)
var<storage, read> input: Pixels;

@group(0)
@binding(1)
var<storage, read_write> output: Pixels;

@compute
@workgroup_size(16,16)

fn main(
    @builtin(global_invocation_id) id: vec3<u32>
) {
    let index = (id.y * 1280u + id.x) * 4u;

    output.data[index] = input.data[index];
    output.data[index + 1u] = input.data[index + 1u];
    output.data[index + 2u] = input.data[index + 2u];
    output.data[index + 3u] = input.data[index + 3u];
}