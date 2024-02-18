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
	time::{frame_to_seconds, seconds_to_frames, seconds_to_frames_i64},
	Visualizer,
};

const FINISHED_SEEK_DETECTION_THRESHOLD: Duration = Duration::from_millis(100);

pub struct MainState {
	visualizer: Box<dyn Visualizer>,
	audio_manager: AudioManager,
	mode: Mode,
	num_frames: u64,
	previous_frame: u64,
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
		let num_frames =
			seconds_to_frames(sound_data.duration().as_secs_f64(), visualizer.frame_rate());
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
				start_frame: 0,
			},
			num_frames,
			previous_frame: 0,
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

	fn current_frame(&self) -> u64 {
		match &self.mode {
			Mode::Stopped { start_frame, .. } => *start_frame,
			Mode::PlayingOrPaused {
				sound,
				in_progress_seek,
			} => in_progress_seek.unwrap_or_else(|| {
				seconds_to_frames(sound.position(), self.visualizer.frame_rate())
			}),
			Mode::Rendering { current_frame, .. } => *current_frame,
		}
	}

	fn play_or_resume(&mut self) -> anyhow::Result<()> {
		match &mut self.mode {
			Mode::Stopped { data, start_frame } => {
				let mut data = data.take().unwrap();
				data.settings.start_position = PlaybackPosition::Seconds(frame_to_seconds(
					*start_frame,
					self.visualizer.frame_rate(),
				));
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

	fn seek(&mut self, frame: u64) -> anyhow::Result<()> {
		match &mut self.mode {
			Mode::Stopped { start_frame, .. } => {
				*start_frame = frame;
			}
			Mode::PlayingOrPaused {
				sound,
				in_progress_seek,
			} => {
				sound.seek_to(frame_to_seconds(frame, self.visualizer.frame_rate()))?;
				*in_progress_seek = Some(frame);
			}
			Mode::Rendering { .. } => unreachable!("not supported in rendering mode"),
		}
		Ok(())
	}

	fn seek_by(&mut self, delta: i64) -> anyhow::Result<()> {
		let frame = (self.current_frame() as i64 + delta).clamp(0, self.num_frames as i64);
		self.seek(frame as u64)
	}

	fn seek_by_seconds(&mut self, delta: f64) -> anyhow::Result<()> {
		let delta_frames = seconds_to_frames_i64(delta, self.visualizer.frame_rate());
		self.seek_by(delta_frames)
	}
}

impl State<anyhow::Error> for MainState {
	fn ui(&mut self, ctx: &mut Context, egui_ctx: &egui::Context) -> Result<(), anyhow::Error> {
		self.render_main_menu(egui_ctx)?;
		self.render_rendering_window(ctx, egui_ctx)?;
		Ok(())
	}

	fn event(&mut self, _ctx: &mut Context, event: Event) -> Result<(), anyhow::Error> {
		if let Event::KeyPressed { key, .. } = event {
			match key {
				Scancode::Space => self.toggle_playback()?,
				Scancode::Left => self.seek_by_seconds(-10.0)?,
				Scancode::Right => self.seek_by_seconds(10.0)?,
				Scancode::Comma => self.go_to_previous_chapter()?,
				Scancode::Period => self.go_to_next_chapter()?,
				_ => {}
			}
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
				let detection_threshold_frames = seconds_to_frames_i64(
					FINISHED_SEEK_DETECTION_THRESHOLD.as_secs_f64(),
					self.visualizer.frame_rate(),
				);
				let sound_position_frames =
					seconds_to_frames_i64(sound.position(), self.visualizer.frame_rate());
				if (sound_position_frames - *in_progress_seek_destination as i64).abs()
					<= detection_threshold_frames
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
					start_frame: 0,
				};
			}
		}
		Ok(())
	}

	fn draw(&mut self, ctx: &mut Context) -> Result<(), anyhow::Error> {
		ctx.clear(LinSrgba::BLACK);
		let current_frame = self.current_frame();
		if current_frame != self.previous_frame {
			let ctx = &mut self.canvas.render_to(ctx);
			self.visualizer.draw(ctx, current_frame)?;
			self.previous_frame = current_frame;
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
				*current_frame += 1;
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
		start_frame: u64,
	},
	PlayingOrPaused {
		sound: StreamingSoundHandle<FromFileError>,
		in_progress_seek: Option<u64>,
	},
	Rendering {
		end_frame: u64,
		current_frame: u64,
		canvas_read_buffer: Vec<u8>,
		ffmpeg_process: Child,
	},
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
struct RenderingSettings {
	start_chapter_index: usize,
	end_chapter_index: usize,
}
