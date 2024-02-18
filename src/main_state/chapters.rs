use super::MainState;

impl MainState {
	pub fn go_to_chapter(&mut self, chapter_index: usize) -> anyhow::Result<()> {
		let Some(chapters) = &self.chapters else {
			return Ok(());
		};
		let chapter = &chapters[chapter_index];
		let chapter_start_position = chapter.start_frame.to_seconds(self.visualizer.frame_rate());
		self.seek(chapter_start_position)?;
		Ok(())
	}

	pub fn go_to_next_chapter(&mut self) -> anyhow::Result<()> {
		let Some(chapters) = &self.chapters else {
			return Ok(());
		};
		let current_chapter_index = chapters
			.index_at_frame(
				self.current_position()
					.to_frames(self.visualizer.frame_rate()),
			)
			.expect("no current chapter");
		if current_chapter_index >= chapters.len() - 1 {
			return Ok(());
		}
		self.go_to_chapter(current_chapter_index + 1)?;
		Ok(())
	}

	pub fn go_to_previous_chapter(&mut self) -> anyhow::Result<()> {
		let Some(chapters) = &self.chapters else {
			return Ok(());
		};
		let current_chapter_index = chapters
			.index_at_frame(
				self.current_position()
					.to_frames(self.visualizer.frame_rate()),
			)
			.expect("no current chapter");
		if current_chapter_index == 0 {
			return Ok(());
		}
		self.go_to_chapter(current_chapter_index - 1)?;
		Ok(())
	}
}
