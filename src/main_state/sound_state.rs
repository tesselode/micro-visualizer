use kira::sound::{
	streaming::{StreamingSoundData, StreamingSoundHandle},
	FromFileError, PlaybackState,
};

#[allow(clippy::large_enum_variant)]
pub enum SoundState {
	Stopped {
		data: Option<StreamingSoundData<FromFileError>>,
		start_position: f64,
	},
	PlayingOrPaused {
		sound: StreamingSoundHandle<FromFileError>,
		in_progress_seek: Option<f64>,
	},
}

impl SoundState {
	pub fn playing(&self) -> bool {
		match self {
			SoundState::Stopped { .. } => false,
			SoundState::PlayingOrPaused { sound, .. } => sound.state() == PlaybackState::Playing,
		}
	}

	pub fn current_position(&self) -> f64 {
		match self {
			SoundState::Stopped { start_position, .. } => *start_position,
			SoundState::PlayingOrPaused {
				sound,
				in_progress_seek,
			} => in_progress_seek.unwrap_or_else(|| sound.position()),
		}
	}
}
