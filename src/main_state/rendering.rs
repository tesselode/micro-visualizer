use std::process::{Command, Stdio};

use kira::sound::streaming::{StreamingSoundData, StreamingSoundSettings};
use micro::{graphics::SwapInterval, Context};
use rfd::FileDialog;

use super::{MainState, Mode};

impl MainState {
	pub fn render(&mut self, ctx: &mut Context) -> anyhow::Result<()> {
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

	pub fn stop_rendering(&mut self, ctx: &mut Context) -> Result<(), anyhow::Error> {
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
