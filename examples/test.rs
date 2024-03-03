use std::path::PathBuf;

use egui::Ui;
use micro::{
	clear,
	graphics::{mesh::Mesh, Canvas, ColorConstants},
	math::Rect,
	with_canvas,
};
use micro_visualizer::{Visualizer, VisualizerInfo};
use palette::LinSrgba;

struct TestVisualizer;

impl TestVisualizer {
	pub fn new() -> anyhow::Result<Self> {
		Ok(Self)
	}
}

impl Visualizer for TestVisualizer {
	fn audio_path(&self) -> PathBuf {
		"test.flac".into()
	}

	fn menu(&mut self, ui: &mut Ui, _vis_info: VisualizerInfo) -> Result<(), anyhow::Error> {
		ui.label("hello!");
		Ok(())
	}

	fn draw(&mut self, vis_info: VisualizerInfo, main_canvas: &Canvas) -> anyhow::Result<()> {
		with_canvas!(main_canvas, {
			clear(LinSrgba::BLACK);
			Mesh::rectangle(Rect::from_xywh(
				50.0 + vis_info.current_frame as f32,
				50.0,
				100.0,
				150.0,
			))
			.draw();
		});
		Ok(())
	}
}

fn main() {
	micro_visualizer::run(TestVisualizer::new);
}
