mod chapters;
mod main_state;
mod time;

pub use chapters::*;

use std::{path::PathBuf, time::Duration};

use glam::UVec2;
use main_state::MainState;
use micro::{Context, ContextSettings, WindowMode};

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
			qualifier: "com",
			organization_name: "tesselode",
			app_name: "micro_visualizer",
			..Default::default()
		},
		|ctx| {
			let visualizer = Box::new(visualizer_constructor(ctx)?);
			MainState::new(ctx, visualizer)
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
		frame_number: u64,
	) -> anyhow::Result<()>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VisualizerInfo {
	pub current_frame: u64,
	pub current_time: Duration,
	pub current_chapter_index: Option<usize>,
}
