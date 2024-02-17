mod ui;

use std::{
	io::Write,
	process::{Child, Command, Stdio},
	time::Duration,
};

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
	graphics::{Canvas, CanvasSettings, ColorConstants, DrawParams, SwapInterval},
	input::Scancode,
	Context, Event, State,
};
use palette::LinSrgba;
use rfd::FileDialog;

use crate::{Chapter, Visualizer};

const FINISHED_SEEK_DETECTION_THRESHOLD: f64 = 0.1;

pub struct MainState {
	visualizer: Box<dyn Visualizer>,
	audio_manager: AudioManager,
	mode: Mode,
	duration: Duration,
	previous_position: f64,
	chapters: Vec<Chapter>,
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
		let duration = sound_data.duration();
		let chapters = visualizer.chapters();
		let canvas = Canvas::new(
			ctx,
			visualizer.video_resolution(),
			CanvasSettings::default(),
		);
		let rendering_settings = if chapters.is_empty() {
			RenderingSettings::default()
		} else {
			RenderingSettings {
				start_chapter_index: 0,
				end_chapter_index: chapters.len() - 1,
			}
		};
		Ok(MainState {
			visualizer,
			audio_manager,
			mode: Mode::Stopped {
				data: Some(sound_data),
				start_position: 0.0,
			},
			duration,
			previous_position: 0.0,
			chapters,
			canvas,
			rendering_settings,
			show_rendering_window: false,
		})
	}

	fn frame_to_time(&self, frame: u64) -> f64 {
		frame as f64 / self.visualizer.frame_rate() as f64
	}

	fn num_frames(&self) -> u64 {
		(self.duration.as_secs_f64() * self.visualizer.frame_rate() as f64).ceil() as u64
	}

	fn playing(&self) -> bool {
		match &self.mode {
			Mode::Stopped { .. } => false,
			Mode::PlayingOrPaused { sound, .. } => sound.state() == PlaybackState::Playing,
			Mode::Rendering { .. } => false,
		}
	}

	fn current_position(&self) -> f64 {
		match &self.mode {
			Mode::Stopped { start_position, .. } => *start_position,
			Mode::PlayingOrPaused {
				sound,
				in_progress_seek,
			} => in_progress_seek.unwrap_or_else(|| sound.position()),
			Mode::Rendering { current_frame, .. } => self.frame_to_time(*current_frame),
		}
	}

	fn play_or_resume(&mut self) -> anyhow::Result<()> {
		match &mut self.mode {
			Mode::Stopped {
				data,
				start_position,
			} => {
				let mut data = data.take().unwrap();
				data.settings.start_position = PlaybackPosition::Seconds(*start_position);
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

	fn seek(&mut self, position: f64) -> anyhow::Result<()> {
		match &mut self.mode {
			Mode::Stopped { start_position, .. } => {
				*start_position = position;
			}
			Mode::PlayingOrPaused {
				sound,
				in_progress_seek,
			} => {
				sound.seek_to(position)?;
				*in_progress_seek = Some(position);
			}
			Mode::Rendering { .. } => unreachable!("not supported in rendering mode"),
		}
		Ok(())
	}

	fn current_frame(&self) -> u64 {
		(self.current_position() * self.visualizer.frame_rate() as f64).ceil() as u64
	}

	fn current_chapter_index(&self) -> Option<usize> {
		self.chapters
			.iter()
			.enumerate()
			.rev()
			.find(|(_, chapter)| chapter.start_frame <= self.current_frame())
			.map(|(index, _)| index)
	}

	fn chapter_end_frame(&self, chapter_index: usize) -> u64 {
		self.chapters
			.get(chapter_index + 1)
			.map(|chapter| chapter.start_frame - 1)
			.unwrap_or(self.num_frames() - 1)
	}

	fn go_to_chapter(&mut self, chapter_index: usize) -> anyhow::Result<()> {
		let chapter = &self.chapters[chapter_index];
		let chapter_start_position = self.frame_to_time(chapter.start_frame);
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

	fn render(&mut self, ctx: &mut Context) -> anyhow::Result<()> {
		let Some(video_path) = FileDialog::new()
			.set_directory(std::env::current_exe().unwrap())
			.add_filter("mp4 video", &["mp4"])
			.save_file()
		else {
			return Ok(());
		};
		let (start_frame, end_frame) = if !self.chapters.is_empty() {
			let start_frame =
				self.chapters[self.rendering_settings.start_chapter_index].start_frame;
			let end_frame = self.chapter_end_frame(self.rendering_settings.end_chapter_index);
			(start_frame, end_frame)
		} else {
			(0, self.num_frames() - 1)
		};
		let start_time = self.frame_to_time(start_frame);
		let ffmpeg_process = Command::new("ffmpeg")
			.stdin(Stdio::piped())
			.arg("-y")
			.arg("-f")
			.arg("rawvideo")
			.arg("-vcodec")
			.arg("rawvideo")
			.arg("-s")
			.arg(&format!(
				"{}x{}",
				self.visualizer.video_resolution().x,
				self.visualizer.video_resolution().y
			))
			.arg("-pix_fmt")
			.arg("rgba")
			.arg("-r")
			.arg(self.visualizer.frame_rate().to_string())
			.arg("-i")
			.arg("-")
			.arg("-ss")
			.arg(&format!("{}s", start_time))
			.arg("-i")
			.arg(self.visualizer.audio_path())
			.arg("-b:a")
			.arg("320k")
			.arg("-c:v")
			.arg("libx264")
			.arg("-r")
			.arg(self.visualizer.frame_rate().to_string())
			.arg("-shortest")
			.arg(video_path)
			.spawn()?;
		let canvas_read_buffer = vec![
			0;
			(self.visualizer.video_resolution().x * self.visualizer.video_resolution().y * 4)
				as usize
		];
		self.mode = Mode::Rendering {
			end_frame,
			current_frame: start_frame,
			canvas_read_buffer,
			ffmpeg_process,
		};
		ctx.set_swap_interval(SwapInterval::Immediate)?;
		Ok(())
	}

	fn stop_rendering(&mut self, ctx: &mut Context) -> Result<(), anyhow::Error> {
		self.mode = Mode::Stopped {
			data: Some(StreamingSoundData::from_file(
				self.visualizer.audio_path(),
				StreamingSoundSettings::default(),
			)?),
			start_position: 0.0,
		};
		ctx.set_swap_interval(SwapInterval::VSync)?;
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
				if (sound.position() - *in_progress_seek_destination).abs()
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
					start_position: 0.0,
				};
			}
		}
		Ok(())
	}

	fn draw(&mut self, ctx: &mut Context) -> Result<(), anyhow::Error> {
		ctx.clear(LinSrgba::BLACK);
		let current_frame = self.current_frame();
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
		start_position: f64,
	},
	PlayingOrPaused {
		sound: StreamingSoundHandle<FromFileError>,
		in_progress_seek: Option<f64>,
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
