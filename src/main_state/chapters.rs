use super::MainState;

impl MainState {
	pub fn current_chapter_index(&self) -> Option<usize> {
		self.chapters
			.iter()
			.enumerate()
			.rev()
			.find(|(_, chapter)| chapter.start_frame <= self.current_frame())
			.map(|(index, _)| index)
	}

	pub fn chapter_end_frame(&self, chapter_index: usize) -> u64 {
		self.chapters
			.get(chapter_index + 1)
			.map(|chapter| chapter.start_frame - 1)
			.unwrap_or(self.num_frames() - 1)
	}

	pub fn go_to_chapter(&mut self, chapter_index: usize) -> anyhow::Result<()> {
		let chapter = &self.chapters[chapter_index];
		let chapter_start_position = self.frame_to_time(chapter.start_frame);
		self.seek(chapter_start_position)?;
		Ok(())
	}

	pub fn go_to_next_chapter(&mut self) -> anyhow::Result<()> {
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

	pub fn go_to_previous_chapter(&mut self) -> anyhow::Result<()> {
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
