use std::path::PathBuf;

use micro::{
	color::{ColorConstants, LinSrgba},
	graphics::{mesh::Mesh, Canvas},
	math::Rect,
	ui::Ui,
	Context,
};
use micro_visualizer::{Visualizer, VisualizerInfo};

struct TestVisualizer;

impl TestVisualizer {
	pub fn new(_ctx: &mut Context) -> anyhow::Result<Self> {
		Ok(Self)
	}
}

impl Visualizer for TestVisualizer {
	fn audio_path(&self) -> PathBuf {
		"test.flac".into()
	}

	fn menu(
		&mut self,
		_ctx: &mut Context,
		ui: &mut Ui,
		_vis_info: VisualizerInfo,
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
			Rect::new((50.0 + vis_info.current_frame as f32, 50.0), (100.0, 150.0)),
		)
		.draw(ctx);
		Ok(())
	}
}

fn main() {
	micro_visualizer::run(TestVisualizer::new);
}
