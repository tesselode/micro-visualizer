mod sound_state;
mod ui;

use std::time::Duration;

use egui::TopBottomPanel;
use glam::Vec2;
use kira::{
	manager::{AudioManager, AudioManagerSettings},
	sound::{
		streaming::{StreamingSoundData, StreamingSoundSettings},
		PlaybackPosition, PlaybackState,
	},
	tween::Tween,
};
use micro::{
	graphics::{Canvas, CanvasSettings, DrawParams},
	Context, State,
};

use crate::{Chapter, Visualizer};

use self::sound_state::SoundState;

const FINISHED_SEEK_DETECTION_THRESHOLD: f64 = 0.1;

pub struct MainState {
	visualizer: Box<dyn Visualizer>,
	audio_manager: AudioManager,
	sound_state: SoundState,
	duration: Duration,
	previous_position: f64,
	chapters: Vec<Chapter>,
	canvas: Canvas,
}

impl MainState {
	pub fn new(ctx: &mut Context, visualizer: Box<dyn Visualizer>) -> anyhow::Result<Self> {
		let audio_manager = AudioManager::new(AudioManagerSettings::default())?;
		let sound_data = StreamingSoundData::from_file(
			visualizer.audio_path(),
			StreamingSoundSettings::default(),
		)?;
		let duration = sound_data.duration();
		let chapters = visualizer.chapters();
		let canvas = Canvas::new(
			ctx,
			visualizer.video_resolution(),
			CanvasSettings::default(),
		);
		Ok(MainState {
			visualizer,
			audio_manager,
			sound_state: SoundState::Stopped {
				data: Some(sound_data),
				start_position: 0.0,
			},
			duration,
			previous_position: 0.0,
			chapters,
			canvas,
		})
	}

	fn play_or_resume(&mut self) -> anyhow::Result<()> {
		match &mut self.sound_state {
			SoundState::Stopped {
				data,
				start_position,
			} => {
				let mut data = data.take().unwrap();
				data.settings.start_position = PlaybackPosition::Seconds(*start_position);
				self.sound_state = SoundState::PlayingOrPaused {
					sound: self.audio_manager.play(data)?,
					in_progress_seek: None,
				};
			}
			SoundState::PlayingOrPaused { sound, .. } => {
				sound.resume(Tween::default())?;
			}
		}
		Ok(())
	}

	fn pause(&mut self) -> anyhow::Result<()> {
		if let SoundState::PlayingOrPaused { sound, .. } = &mut self.sound_state {
			sound.pause(Tween::default())?;
		}
		Ok(())
	}

	fn seek(&mut self, position: f64) -> anyhow::Result<()> {
		match &mut self.sound_state {
			SoundState::Stopped { start_position, .. } => {
				*start_position = position;
			}
			SoundState::PlayingOrPaused {
				sound,
				in_progress_seek,
			} => {
				sound.seek_to(position)?;
				*in_progress_seek = Some(position);
			}
		}
		Ok(())
	}
}

impl State<anyhow::Error> for MainState {
	fn ui(&mut self, _ctx: &mut Context, egui_ctx: &egui::Context) -> Result<(), anyhow::Error> {
		TopBottomPanel::bottom("main_menu")
			.show(egui_ctx, |ui| -> anyhow::Result<()> {
				egui::menu::bar(ui, |ui| -> anyhow::Result<()> {
					self.render_play_pause_button(ui)?;
					self.render_seekbar(ui)?;
					Ok(())
				})
				.inner
			})
			.inner?;
		Ok(())
	}

	fn update(&mut self, _ctx: &mut Context, _delta_time: Duration) -> Result<(), anyhow::Error> {
		if let SoundState::PlayingOrPaused {
			sound,
			in_progress_seek,
		} = &mut self.sound_state
		{
			if let Some(in_progress_seek_destination) = in_progress_seek {
				if (sound.position() - *in_progress_seek_destination).abs()
					<= FINISHED_SEEK_DETECTION_THRESHOLD
				{
					*in_progress_seek = None;
				}
			}
			if sound.state() == PlaybackState::Stopped {
				self.sound_state = SoundState::Stopped {
					data: Some(StreamingSoundData::from_file(
						self.visualizer.audio_path(),
						StreamingSoundSettings::default(),
					)?),
					start_position: 0.0,
				};
			}
		}
		Ok(())
	}

	fn draw(&mut self, ctx: &mut Context) -> Result<(), anyhow::Error> {
		if self.sound_state.current_position() != self.previous_position {
			let ctx = &mut self.canvas.render_to(ctx);
			let current_frame = (self.sound_state.current_position()
				* self.visualizer.frame_rate() as f64)
				.ceil() as u64;
			self.visualizer.draw(ctx, current_frame)?;
			self.previous_position = self.sound_state.current_position();
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
		Ok(())
	}
}
