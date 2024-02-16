mod sound_state;
mod ui;

use std::time::Duration;

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
	graphics::{Canvas, CanvasSettings, ColorConstants, DrawParams},
	input::Scancode,
	Context, Event, State,
};
use palette::LinSrgba;

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

	fn toggle_playback(&mut self) -> anyhow::Result<()> {
		if self.sound_state.playing() {
			self.pause()?;
		} else {
			self.play_or_resume()?;
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

	fn current_frame(&self) -> u64 {
		(self.sound_state.current_position() * self.visualizer.frame_rate() as f64).ceil() as u64
	}

	fn current_chapter_index(&self) -> Option<usize> {
		self.chapters
			.iter()
			.enumerate()
			.rev()
			.find(|(_, chapter)| chapter.start_frame <= self.current_frame())
			.map(|(index, _)| index)
	}

	fn go_to_chapter(&mut self, chapter_index: usize) -> anyhow::Result<()> {
		let chapter = &self.chapters[chapter_index];
		let chapter_start_position =
			chapter.start_frame as f64 / self.visualizer.frame_rate() as f64;
		self.seek(chapter_start_position)?;
		Ok(())
	}

	fn go_to_next_chapter(&mut self) -> anyhow::Result<()> {
		if self.chapters.is_empty() {
			return Ok(());
		}
		let current_chapter_index = self.current_chapter_index().expect("no current chapter");
		if current_chapter_index >= self.chapters.len() - 1 {
			return Ok(());
		}
		self.go_to_chapter(current_chapter_index + 1)?;
		Ok(())
	}

	fn go_to_previous_chapter(&mut self) -> anyhow::Result<()> {
		if self.chapters.is_empty() {
			return Ok(());
		}
		let current_chapter_index = self.current_chapter_index().expect("no current chapter");
		if current_chapter_index == 0 {
			return Ok(());
		}
		self.go_to_chapter(current_chapter_index - 1)?;
		Ok(())
	}
}

impl State<anyhow::Error> for MainState {
	fn ui(&mut self, _ctx: &mut Context, egui_ctx: &egui::Context) -> Result<(), anyhow::Error> {
		self.render_main_menu(egui_ctx)?;
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
		ctx.clear(LinSrgba::BLACK);
		let current_frame = self.current_frame();
		if self.sound_state.current_position() != self.previous_position {
			let ctx = &mut self.canvas.render_to(ctx);
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
