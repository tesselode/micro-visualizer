use std::time::Duration;

use egui::{Slider, TopBottomPanel};
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
	graphics::{Canvas, CanvasSettings, DrawParams},
	Context, State,
};

use crate::{Chapter, Visualizer};

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
				};
			}
			SoundState::PlayingOrPaused { sound } => {
				sound.resume(Tween::default())?;
			}
		}
		Ok(())
	}

	fn pause(&mut self) -> anyhow::Result<()> {
		if let SoundState::PlayingOrPaused { sound } = &mut self.sound_state {
			sound.pause(Tween::default())?;
		}
		Ok(())
	}

	fn seek(&mut self, position: f64) -> anyhow::Result<()> {
		match &mut self.sound_state {
			SoundState::Stopped { start_position, .. } => {
				*start_position = position;
			}
			SoundState::PlayingOrPaused { sound } => {
				sound.seek_to(position)?;
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
					let play_pause_button_text = if self.sound_state.playing() {
						"Pause"
					} else {
						"Play"
					};
					if ui.button(play_pause_button_text).clicked() {
						if self.sound_state.playing() {
							self.pause()?;
						} else {
							self.play_or_resume()?;
						}
					}
					let mut position = self.sound_state.current_position();
					if ui
						.add(
							Slider::new(&mut position, 0.0..=self.duration.as_secs_f64())
								.custom_formatter(|position, _| format_position(position)),
						)
						.drag_released()
					{
						self.seek(position)?;
					};
					Ok(())
				})
				.inner
			})
			.inner?;
		Ok(())
	}

	fn update(&mut self, _ctx: &mut Context, _delta_time: Duration) -> Result<(), anyhow::Error> {
		if let SoundState::PlayingOrPaused { sound } = &self.sound_state {
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

#[allow(clippy::large_enum_variant)]
enum SoundState {
	Stopped {
		data: Option<StreamingSoundData<FromFileError>>,
		start_position: f64,
	},
	PlayingOrPaused {
		sound: StreamingSoundHandle<FromFileError>,
	},
}

impl SoundState {
	fn playing(&self) -> bool {
		match self {
			SoundState::Stopped { .. } => false,
			SoundState::PlayingOrPaused { sound } => sound.state() == PlaybackState::Playing,
		}
	}

	fn current_position(&self) -> f64 {
		match self {
			SoundState::Stopped { start_position, .. } => *start_position,
			SoundState::PlayingOrPaused { sound } => sound.position(),
		}
	}
}

fn format_position(position: f64) -> String {
	let seconds = position % 60.0;
	let minutes = (position / 60.0).floor() % 60.0;
	let hours = (position / (60.0 * 60.0)).floor();
	format!("{}:{:0>2}:{:0>5.2}", hours, minutes, seconds)
}