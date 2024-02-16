use egui::{ComboBox, Slider, TopBottomPanel, Ui};

use super::MainState;

impl MainState {
	pub fn render_main_menu(&mut self, egui_ctx: &egui::Context) -> Result<(), anyhow::Error> {
		TopBottomPanel::bottom("main_menu")
			.show(egui_ctx, |ui| -> anyhow::Result<()> {
				egui::menu::bar(ui, |ui| -> anyhow::Result<()> {
					self.render_play_pause_button(ui)?;
					self.render_seekbar(ui)?;
					self.render_chapter_combo_box(ui)?;
					if ui.button("<<").clicked() {
						self.go_to_previous_chapter()?;
					}
					if ui.button(">>").clicked() {
						self.go_to_next_chapter()?;
					}
					Ok(())
				})
				.inner
			})
			.inner?;
		Ok(())
	}

	fn render_play_pause_button(&mut self, ui: &mut Ui) -> Result<(), anyhow::Error> {
		let play_pause_button_text = if self.sound_state.playing() {
			"Pause"
		} else {
			"Play"
		};
		if ui.button(play_pause_button_text).clicked() {
			self.toggle_playback()?;
		};
		Ok(())
	}

	fn render_seekbar(&mut self, ui: &mut Ui) -> Result<(), anyhow::Error> {
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

	fn render_chapter_combo_box(&mut self, ui: &mut Ui) -> anyhow::Result<()> {
		if self.chapters.is_empty() {
			return Ok(());
		}
		let mut selected = self.current_chapter_index().expect("no current chapter");
		let response =
			ComboBox::new("chapter", "").show_index(ui, &mut selected, self.chapters.len(), |i| {
				&self.chapters[i].name
			});
		if response.changed() {
			self.go_to_chapter(selected)?;
		}
		Ok(())
	}
}

fn format_position(position: f64) -> String {
	let seconds = position % 60.0;
	let minutes = (position / 60.0).floor() % 60.0;
	let hours = (position / (60.0 * 60.0)).floor();
	format!("{}:{:0>2}:{:0>5.2}", hours, minutes, seconds)
}
