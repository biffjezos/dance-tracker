/*
Same graph shape as the original stub demo (two VideoSource nodes into
one Compose node). Native binary, so sources here use a fixed-Frame
PixelSource stand-in for the real DOM-backed one (dom.rs, wasm32 only)
- proves the executor -> operation wiring produces a correct composited
pixel, not just that it runs without crashing.
*/

use dance_tracker_core::compositor::{Context, OperationError};
use dance_tracker_core::graph::{Graph, Node};
use dance_tracker_core::operations::composite::{BlendMode, Compose};
use dance_tracker_core::operations::executor::{Execute, SimpleExecutor};
use dance_tracker_core::operations::sources::video::VideoSource;
use dance_tracker_core::operations::sources::PixelSource;
use dance_tracker_core::operations::{expect_frame, Frame};

struct FixedPixelSource(Frame);

impl PixelSource for FixedPixelSource {
    fn read(&self) -> Result<Frame, OperationError> {
        Ok(self.0.clone())
    }
}

fn main() {
    let mut graph = Graph { nodes: vec![] };

    let keyed_source = VideoSource {
        pixels: Box::new(FixedPixelSource(Frame {
            pixels: vec![255, 0, 0, 128], // semi-transparent red, 1x1
            width: 1,
            height: 1,
            timestamp: 0.0,
        })),
    };
    let keyed_source_node = Node {
        operation: Box::new(keyed_source),
        inputs: vec![],
    };
    let keyed_video_source_id = graph.add_node(keyed_source_node);

    let backdrop_source = VideoSource {
        pixels: Box::new(FixedPixelSource(Frame {
            pixels: vec![0, 0, 255, 255], // opaque blue, 1x1
            width: 1,
            height: 1,
            timestamp: 0.0,
        })),
    };
    let backdrop_source_node = Node {
        operation: Box::new(backdrop_source),
        inputs: vec![],
    };
    let backdrop_source_node_id = graph.add_node(backdrop_source_node);

    let compose = Compose {
        mode: BlendMode::Over,
    };
    let compose_node = Node {
        operation: Box::new(compose),
        inputs: vec![keyed_video_source_id, backdrop_source_node_id],
    };
    let compose_id = graph.add_node(compose_node);

    println!("Graph contains {:?} nodes", graph.nodes.len());

    let ctx = Context { data: Box::new(()) };

    let executor = SimpleExecutor;
    let values = executor
        .execute(&graph, compose_id, &ctx)
        .expect("compose should succeed");

    let frame = expect_frame(values.first()).expect("compose should output a Frame");

    println!(
        "Composed 1x1 pixel: rgba({}, {}, {}, {})",
        frame.pixels[0], frame.pixels[1], frame.pixels[2], frame.pixels[3]
    );
}
