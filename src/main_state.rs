mod chapters;
mod rendering;
mod ui;

use std::{io::Write, process::Child, time::Duration};

use glam::Vec2;
use kira::{
	manager::{AudioManager, AudioManagerSettings},
	sound::{
		streaming::{StreamingSoundData, StreamingSoundHandle, StreamingSoundSettings},
		FromFileError, PlaybackPosition, PlaybackState,
	},
	tween::Tween,
};
use micro::{
	graphics::{Canvas, CanvasSettings, ColorConstants, DrawParams},
	input::Scancode,
	Context, Event, State,
};
use palette::LinSrgba;

use crate::{
	chapters::Chapters,
	time::{Frames, Seconds},
	Visualizer,
};

const FINISHED_SEEK_DETECTION_THRESHOLD: Seconds = Seconds(0.1);

pub struct MainState {
	visualizer: Box<dyn Visualizer>,
	audio_manager: AudioManager,
	mode: Mode,
	duration: Seconds,
	previous_position: Seconds,
	chapters: Option<Chapters>,
	canvas: Canvas,
	rendering_settings: RenderingSettings,
	show_rendering_window: bool,
}

impl MainState {
	pub fn new(ctx: &mut Context, visualizer: Box<dyn Visualizer>) -> anyhow::Result<Self> {
		let audio_manager = AudioManager::new(AudioManagerSettings::default())?;
		let sound_data = StreamingSoundData::from_file(
			visualizer.audio_path(),
			StreamingSoundSettings::default(),
		)?;
		let duration = Seconds(sound_data.duration().as_secs_f64());
		let chapters = visualizer.chapters();
		let chapters = if visualizer.chapters().is_empty() {
			None
		} else {
			Some(Chapters(chapters))
		};
		let canvas = Canvas::new(
			ctx,
			visualizer.video_resolution(),
			CanvasSettings::default(),
		);
		let rendering_settings = if let Some(chapters) = &chapters {
			RenderingSettings {
				start_chapter_index: 0,
				end_chapter_index: chapters.len() - 1,
			}
		} else {
			RenderingSettings::default()
		};
		Ok(MainState {
			visualizer,
			audio_manager,
			mode: Mode::Stopped {
				data: Some(sound_data),
				start_position: Seconds(0.0),
			},
			duration,
			previous_position: Seconds(0.0),
			chapters,
			canvas,
			rendering_settings,
			show_rendering_window: false,
		})
	}

	fn playing(&self) -> bool {
		match &self.mode {
			Mode::Stopped { .. } => false,
			Mode::PlayingOrPaused { sound, .. } => sound.state() == PlaybackState::Playing,
			Mode::Rendering { .. } => false,
		}
	}

	fn current_position(&self) -> Seconds {
		match &self.mode {
			Mode::Stopped { start_position, .. } => *start_position,
			Mode::PlayingOrPaused {
				sound,
				in_progress_seek,
			} => in_progress_seek.unwrap_or_else(|| Seconds(sound.position())),
			Mode::Rendering { current_frame, .. } => {
				current_frame.to_seconds(self.visualizer.frame_rate())
			}
		}
	}

	fn play_or_resume(&mut self) -> anyhow::Result<()> {
		match &mut self.mode {
			Mode::Stopped {
				data,
				start_position,
			} => {
				let mut data = data.take().unwrap();
				data.settings.start_position = PlaybackPosition::Seconds(start_position.0);
				self.mode = Mode::PlayingOrPaused {
					sound: self.audio_manager.play(data)?,
					in_progress_seek: None,
				};
			}
			Mode::PlayingOrPaused { sound, .. } => {
				sound.resume(Tween::default())?;
			}
			Mode::Rendering { .. } => unreachable!("not supported in rendering mode"),
		}
		Ok(())
	}

	fn pause(&mut self) -> anyhow::Result<()> {
		if let Mode::PlayingOrPaused { sound, .. } = &mut self.mode {
			sound.pause(Tween::default())?;
		}
		Ok(())
	}

	fn toggle_playback(&mut self) -> anyhow::Result<()> {
		if self.playing() {
			self.pause()?;
		} else {
			self.play_or_resume()?;
		}
		Ok(())
	}

	fn seek(&mut self, position: Seconds) -> anyhow::Result<()> {
		match &mut self.mode {
			Mode::Stopped { start_position, .. } => {
				*start_position = position;
			}
			Mode::PlayingOrPaused {
				sound,
				in_progress_seek,
			} => {
				sound.seek_to(position.0)?;
				*in_progress_seek = Some(position);
			}
			Mode::Rendering { .. } => unreachable!("not supported in rendering mode"),
		}
		Ok(())
	}
}

impl State<anyhow::Error> for MainState {
	fn ui(&mut self, ctx: &mut Context, egui_ctx: &egui::Context) -> Result<(), anyhow::Error> {
		self.render_main_menu(egui_ctx)?;
		self.render_rendering_window(ctx, egui_ctx)?;
		Ok(())
	}

	fn event(&mut self, _ctx: &mut Context, event: Event) -> Result<(), anyhow::Error> {
		if let Event::KeyPressed {
			key: Scancode::Space,
			..
		} = event
		{
			self.toggle_playback()?;
		}
		if let Event::KeyPressed {
			key: Scancode::Comma,
			..
		} = event
		{
			self.go_to_previous_chapter()?;
		}
		if let Event::KeyPressed {
			key: Scancode::Period,
			..
		} = event
		{
			self.go_to_next_chapter()?;
		}
		Ok(())
	}

	fn update(&mut self, _ctx: &mut Context, _delta_time: Duration) -> Result<(), anyhow::Error> {
		if let Mode::PlayingOrPaused {
			sound,
			in_progress_seek,
		} = &mut self.mode
		{
			if let Some(in_progress_seek_destination) = in_progress_seek {
				if (Seconds(sound.position()) - *in_progress_seek_destination).abs()
					<= FINISHED_SEEK_DETECTION_THRESHOLD
				{
					*in_progress_seek = None;
				}
			}
			if sound.state() == PlaybackState::Stopped {
				self.mode = Mode::Stopped {
					data: Some(StreamingSoundData::from_file(
						self.visualizer.audio_path(),
						StreamingSoundSettings::default(),
					)?),
					start_position: Seconds(0.0),
				};
			}
		}
		Ok(())
	}

	fn draw(&mut self, ctx: &mut Context) -> Result<(), anyhow::Error> {
		ctx.clear(LinSrgba::BLACK);
		let current_frame = self
			.current_position()
			.to_frames(self.visualizer.frame_rate());
		if self.current_position() != self.previous_position {
			let ctx = &mut self.canvas.render_to(ctx);
			self.visualizer.draw(ctx, current_frame)?;
			self.previous_position = self.current_position();
		}
		let max_horizontal_scale =
			ctx.window_size().x as f32 / self.visualizer.video_resolution().x as f32;
		let max_vertical_scale =
			ctx.window_size().y as f32 / self.visualizer.video_resolution().y as f32;
		let scale = max_horizontal_scale.min(max_vertical_scale);
		self.canvas.draw(
			ctx,
			DrawParams::new()
				.translated_2d(-self.visualizer.video_resolution().as_vec2() / 2.0)
				.scaled_2d(Vec2::splat(scale))
				.translated_2d(ctx.window_size().as_vec2() / 2.0),
		);
		if let Mode::Rendering {
			end_frame,
			current_frame,
			canvas_read_buffer,
			ffmpeg_process,
		} = &mut self.mode
		{
			self.canvas.read(ctx, canvas_read_buffer);
			let ffmpeg_stdin = ffmpeg_process.stdin.as_mut().unwrap();
			let write_result = ffmpeg_stdin.write_all(canvas_read_buffer);
			if write_result.is_err() {
				self.stop_rendering(ctx)?;
			} else {
				*current_frame += Frames(1);
				if *current_frame > *end_frame {
					self.stop_rendering(ctx)?;
				}
			}
		}
		Ok(())
	}
}

#[allow(clippy::large_enum_variant)]
enum Mode {
	Stopped {
		data: Option<StreamingSoundData<FromFileError>>,
		start_position: Seconds,
	},
	PlayingOrPaused {
		sound: StreamingSoundHandle<FromFileError>,
		in_progress_seek: Option<Seconds>,
	},
	Rendering {
		end_frame: Frames,
		current_frame: Frames,
		canvas_read_buffer: Vec<u8>,
		ffmpeg_process: Child,
	},
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
struct RenderingSettings {
	start_chapter_index: usize,
	end_chapter_index: usize,
}
