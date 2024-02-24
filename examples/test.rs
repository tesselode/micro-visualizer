use std::path::PathBuf;

use egui::Ui;
use micro::{
	graphics::{mesh::Mesh, Canvas, ColorConstants, DrawParams},
	math::Rect,
	Context,
};
use micro_visualizer::{Visualizer, VisualizerInfo};
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

	fn menu(
		&mut self,
		ctx: &mut Context,
		ui: &mut Ui,
		vis_info: VisualizerInfo,
	) -> Result<(), anyhow::Error> {
		ui.label("hello!");
		Ok(())
	}

	fn draw(
		&mut self,
		ctx: &mut Context,
		vis_info: VisualizerInfo,
		main_canvas: &Canvas,
	) -> anyhow::Result<()> {
		let ctx = &mut main_canvas.render_to(ctx);
		ctx.clear(LinSrgba::BLACK);
		Mesh::rectangle(
			ctx,
			Rect::from_xywh(50.0 + vis_info.current_frame as f32, 50.0, 100.0, 150.0),
		)
		.draw(ctx, DrawParams::new());
		Ok(())
	}
}

fn main() {
	micro_visualizer::run(TestVisualizer::new);
}
