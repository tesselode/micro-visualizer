use std::path::PathBuf;

use micro::{
	graphics::{mesh::Mesh, ColorConstants, DrawParams},
	math::Rect,
	Context,
};
use micro_visualizer::Visualizer;
use palette::LinSrgba;

struct TestVisualizer;

impl TestVisualizer {
	pub fn new(ctx: &mut Context) -> anyhow::Result<Self> {
		Ok(Self)
	}
}

impl Visualizer for TestVisualizer {
	fn audio_path(&self) -> PathBuf {
		"test.flac".into()
	}

	fn draw(&mut self, ctx: &mut Context, frame_number: u64) -> anyhow::Result<()> {
		ctx.clear(LinSrgba::BLACK);
		Mesh::rectangle(
			ctx,
			Rect::from_xywh(50.0 + frame_number as f32, 50.0, 100.0, 150.0),
		)
		.draw(ctx, DrawParams::new());
		Ok(())
	}
}

fn main() {
	micro_visualizer::run(TestVisualizer::new);
}
