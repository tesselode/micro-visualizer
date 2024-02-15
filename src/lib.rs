mod main_state;

use std::path::PathBuf;

use glam::UVec2;
use main_state::MainState;
use micro::{Context, ContextSettings, WindowMode};

pub trait Visualizer: 'static {
	fn audio_path(&self) -> PathBuf;

	fn frame_rate(&self) -> u32 {
		60
	}

	fn video_resolution(&self) -> UVec2 {
		UVec2::new(3840, 2160)
	}

	fn chapters(&self) -> Vec<Chapter> {
		vec![]
	}

	fn draw(&mut self, ctx: &mut Context, frame_number: u64) -> anyhow::Result<()>;
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Chapter {
	pub name: String,
	pub start_frame: u64,
}

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
