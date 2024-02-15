use std::time::Duration;

use egui::{Slider, TopBottomPanel};
use glam::Vec2;
use kira::{
	manager::{AudioManager, AudioManagerSettings},
	sound::static_sound::{StaticSoundData, StaticSoundHandle, StaticSoundSettings},
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
	sound_data: StaticSoundData,
	num_frames: u64,
	playback_state: PlaybackState,
	current_frame: u64,
	chapters: Vec<Chapter>,
	canvas: Canvas,
}

impl MainState {
	pub fn new(ctx: &mut Context, visualizer: Box<dyn Visualizer>) -> anyhow::Result<Self> {
		let audio_manager = AudioManager::new(AudioManagerSettings::default())?;
		let sound_data =
			StaticSoundData::from_file(visualizer.audio_path(), StaticSoundSettings::default())?;
		let num_frames =
			(sound_data.duration().as_secs_f64() * visualizer.frame_rate() as f64).ceil() as u64;
		let chapters = visualizer.chapters();
		let canvas = Canvas::new(
			ctx,
			visualizer.video_resolution(),
			CanvasSettings::default(),
		);
		Ok(MainState {
			visualizer,
			audio_manager,
			sound_data,
			num_frames,
			playback_state: PlaybackState::Paused,
			current_frame: 0,
			chapters,
			canvas,
		})
	}

	fn play(&mut self) -> anyhow::Result<()> {
		let start_position = self.current_frame as f64 / self.visualizer.frame_rate() as f64;
		self.playback_state = PlaybackState::Playing {
			sound: self.audio_manager.play(
				self.sound_data
					.with_modified_settings(|s| s.playback_region(start_position..)),
			)?,
			time_since_last_frame: Duration::ZERO,
		};
		Ok(())
	}

	fn pause(&mut self) -> anyhow::Result<()> {
		if let PlaybackState::Playing { sound, .. } = &mut self.playback_state {
			sound.stop(Tween::default())?;
		}
		self.playback_state = PlaybackState::Paused;
		Ok(())
	}

	fn toggle_playback(&mut self) -> anyhow::Result<()> {
		match &self.playback_state {
			PlaybackState::Playing { .. } => self.pause(),
			PlaybackState::Paused => self.play(),
		}
	}

	fn seek(&mut self, frame: u64) -> Result<(), anyhow::Error> {
		self.current_frame = frame;
		if let PlaybackState::Playing { sound, .. } = &mut self.playback_state {
			sound.stop(Tween::default())?;
		}
		self.play()?;
		Ok(())
	}
}

impl State<anyhow::Error> for MainState {
	fn ui(&mut self, _ctx: &mut Context, egui_ctx: &egui::Context) -> Result<(), anyhow::Error> {
		TopBottomPanel::bottom("main_menu")
			.show(egui_ctx, |ui| -> anyhow::Result<()> {
				egui::menu::bar(ui, |ui| -> anyhow::Result<()> {
					let play_pause_button_text = match &self.playback_state {
						PlaybackState::Playing { .. } => "Pause",
						PlaybackState::Paused => "Play",
					};
					if ui.button(play_pause_button_text).clicked() {
						self.toggle_playback()?;
					}
					let mut current_frame = self.current_frame;
					if ui
						.add(Slider::new(&mut current_frame, 0..=self.num_frames - 1))
						.drag_released()
					{
						self.seek(current_frame)?;
					};
					Ok(())
				})
				.inner
			})
			.inner?;
		Ok(())
	}

	fn update(&mut self, _ctx: &mut Context, delta_time: Duration) -> Result<(), anyhow::Error> {
		if let PlaybackState::Playing {
			sound,
			time_since_last_frame,
		} = &mut self.playback_state
		{
			if sound.state() == kira::sound::PlaybackState::Stopped {
				self.playback_state = PlaybackState::Paused;
			} else {
				*time_since_last_frame += delta_time;
				let frame_time = Duration::from_secs_f64(1.0 / self.visualizer.frame_rate() as f64);
				while *time_since_last_frame >= frame_time {
					*time_since_last_frame -= frame_time;
					self.current_frame = (self.current_frame + 1).min(self.num_frames - 1);
				}
			}
		}
		Ok(())
	}

	fn draw(&mut self, ctx: &mut Context) -> Result<(), anyhow::Error> {
		{
			let ctx = &mut self.canvas.render_to(ctx);
			self.visualizer.draw(ctx, self.current_frame)?;
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

enum PlaybackState {
	Playing {
		sound: StaticSoundHandle,
		time_since_last_frame: Duration,
	},
	Paused,
}
