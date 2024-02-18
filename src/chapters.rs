use derive_more::{Index, IndexMut, IntoIterator};

use crate::Chapter;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Index, IndexMut, IntoIterator)]
pub struct Chapters(pub Vec<Chapter>);

impl Chapters {
	pub fn get(&self, index: usize) -> Option<&Chapter> {
		self.0.get(index)
	}

	pub fn get_mut(&mut self, index: usize) -> Option<&mut Chapter> {
		self.0.get_mut(index)
	}

	pub fn len(&self) -> usize {
		self.0.len()
	}

	pub fn index_at_frame(&self, frame: u64) -> Option<usize> {
		self.0
			.iter()
			.enumerate()
			.rev()
			.find(|(_, chapter)| chapter.start_frame <= frame)
			.map(|(i, _)| i)
	}

	pub fn at_frame(&self, frame: u64) -> Option<&Chapter> {
		self.0
			.iter()
			.rev()
			.find(|chapter| chapter.start_frame <= frame)
	}

	pub fn end_frame(&self, chapter_index: usize) -> Option<u64> {
		self.get(chapter_index + 1)
			.map(|chapter| chapter.start_frame - 1)
	}
}
