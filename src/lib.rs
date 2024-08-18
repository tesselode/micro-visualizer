mod chapters;
mod time;
mod vis_runner;

pub use chapters::*;

use std::{path::PathBuf, time::Duration};

use micro::{graphics::Canvas, math::UVec2, ui::Ui, Context, ContextSettings, Event, WindowMode};
use vis_runner::VisRunner;

pub fn run<T: Visualizer>(
	mut visualizer_constructor: impl FnMut(&mut Context) -> anyhow::Result<T>,
) {
	micro::run(
		ContextSettings {
			window_title: "Micro Visualizer".into(),
			window_mode: WindowMode::Windowed {
				size: UVec2::new(1280, 720),
			},
			resizable: true,
			..Default::default()
		},
		|ctx| {
			let visualizer = Box::new(visualizer_constructor(ctx)?);
			VisRunner::new(ctx, visualizer)
		},
	)
}

#[allow(unused_variables)]
pub trait Visualizer: 'static {
	fn audio_path(&self) -> PathBuf;

	fn frame_rate(&self) -> u64 {
		60
	}

	fn video_resolution(&self) -> UVec2 {
		UVec2::new(3840, 2160)
	}

	fn chapters(&self) -> Option<&Chapters> {
		None
	}

	fn ui(
		&mut self,
		ctx: &mut Context,
		egui_ctx: &micro::ui::Context,
		vis_info: VisualizerInfo,
	) -> Result<(), anyhow::Error> {
		Ok(())
	}

	fn menu(
		&mut self,
		ctx: &mut Context,
		ui: &mut Ui,
		vis_info: VisualizerInfo,
	) -> Result<(), anyhow::Error> {
		Ok(())
	}

	fn event(
		&mut self,
		ctx: &mut Context,
		vis_info: VisualizerInfo,
		event: Event,
	) -> Result<(), anyhow::Error> {
		Ok(())
	}

	fn update(
		&mut self,
		ctx: &mut Context,
		vis_info: VisualizerInfo,
		delta_time: Duration,
	) -> anyhow::Result<()> {
		Ok(())
	}

	fn draw(
		&mut self,
		ctx: &mut Context,
		vis_info: VisualizerInfo,
		main_canvas: &Canvas,
	) -> anyhow::Result<()>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VisualizerInfo {
	pub resolution: UVec2,
	pub current_frame: u64,
	pub current_time: Duration,
	pub current_chapter_index: Option<usize>,
}
