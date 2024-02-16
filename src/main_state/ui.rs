use egui::Slider;

use super::MainState;

impl MainState {
	pub fn render_play_pause_button(&mut self, ui: &mut egui::Ui) -> Result<(), anyhow::Error> {
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
		};
		Ok(())
	}

	pub fn render_seekbar(&mut self, ui: &mut egui::Ui) -> Result<(), anyhow::Error> {
		let mut position = self.sound_state.current_position();
		if ui
			.add(
				Slider::new(&mut position, 0.0..=self.duration.as_secs_f64()).custom_formatter(
					|position, _| {
						format!(
							"{} / {}",
							format_position(position),
							format_position(self.duration.as_secs_f64())
						)
					},
				),
			)
			.drag_released()
		{
			self.seek(position)?;
		};
		Ok(())
	}
}

fn format_position(position: f64) -> String {
	let seconds = position % 60.0;
	let minutes = (position / 60.0).floor() % 60.0;
	let hours = (position / (60.0 * 60.0)).floor();
	format!("{}:{:0>2}:{:0>5.2}", hours, minutes, seconds)
}
