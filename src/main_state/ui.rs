use egui::{ComboBox, InnerResponse, Slider, TopBottomPanel, Ui};
use micro::Context;

use crate::Seconds;

use super::{MainState, Mode};

impl MainState {
	pub fn render_main_menu(&mut self, egui_ctx: &egui::Context) -> Result<(), anyhow::Error> {
		TopBottomPanel::bottom("main_menu")
			.show(egui_ctx, |ui| -> anyhow::Result<()> {
				egui::menu::bar(ui, |ui| -> anyhow::Result<()> {
					self.render_play_pause_button(ui)?;
					if ui.button("Render").clicked() {
						self.show_rendering_window = true;
					}
					self.render_seekbar(ui)?;
					self.render_chapter_combo_box(ui)?;
					if !matches!(self.mode, Mode::Rendering { .. }) {
						if ui.button("<<").clicked() {
							self.go_to_previous_chapter()?;
						}
						if ui.button(">>").clicked() {
							self.go_to_next_chapter()?;
						}
					}
					Ok(())
				})
				.inner
			})
			.inner?;
		Ok(())
	}

	pub fn render_rendering_window(
		&mut self,
		ctx: &mut Context,
		egui_ctx: &egui::Context,
	) -> anyhow::Result<()> {
		let response = egui::Window::new("Rendering")
			.open(&mut self.show_rendering_window)
			.show(egui_ctx, |ui| {
				let mut rendering_started = false;
				if let Some(chapters) = &self.chapters {
					ComboBox::new("start_chapter_index", "Start Chapter Index").show_index(
						ui,
						&mut self.rendering_settings.start_chapter_index,
						chapters.len(),
						|i| &chapters[i].name,
					);
					ComboBox::new("end_chapter_index", "End Chapter Index").show_index(
						ui,
						&mut self.rendering_settings.end_chapter_index,
						chapters.len(),
						|i| &chapters[i].name,
					);
				}
				if ui.button("Render").clicked() {
					rendering_started = true;
				}
				rendering_started
			});
		if let Some(InnerResponse {
			inner: Some(true), ..
		}) = response
		{
			self.render(ctx)?;
		}
		Ok(())
	}

	fn render_play_pause_button(&mut self, ui: &mut Ui) -> Result<(), anyhow::Error> {
		if matches!(self.mode, Mode::Rendering { .. }) {
			return Ok(());
		}
		let play_pause_button_text = if self.playing() { "Pause" } else { "Play" };
		if ui.button(play_pause_button_text).clicked() {
			self.toggle_playback()?;
		};
		Ok(())
	}

	fn render_seekbar(&mut self, ui: &mut Ui) -> Result<(), anyhow::Error> {
		let mut position = self.current_position().0;
		let slider_response = &ui.add(
			Slider::new(&mut position, 0.0..=self.duration.0).custom_formatter(|position, _| {
				format!(
					"{} / {}",
					format_position(position),
					format_position(self.duration.0)
				)
			}),
		);
		if slider_response.drag_released() && !matches!(self.mode, Mode::Rendering { .. }) {
			self.seek(Seconds(position))?;
		};
		Ok(())
	}

	fn render_chapter_combo_box(&mut self, ui: &mut Ui) -> anyhow::Result<()> {
		let Some(chapters) = &self.chapters else {
			return Ok(());
		};
		let current_frame = self
			.current_position()
			.to_frames(self.visualizer.frame_rate());
		let current_chapter_index = chapters
			.index_at_frame(current_frame)
			.expect("no current chapter");
		if matches!(self.mode, Mode::Rendering { .. }) {
			ui.label(&chapters[current_chapter_index].name);
		} else {
			let mut selected = current_chapter_index;
			let response =
				ComboBox::new("chapter", "")
					.show_index(ui, &mut selected, chapters.len(), |i| &chapters[i].name);
			if response.changed() {
				self.go_to_chapter(selected)?;
			}
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
