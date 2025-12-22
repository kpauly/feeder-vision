//! Settings panel rendering for thresholds, uploads, and updates.

use super::{Panel, UiApp};
use crate::i18n::LanguagePreference;
use eframe::egui;

impl UiApp {
    /// Renders the settings screen including thresholds and telemetry toggles.
    pub(super) fn render_settings_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading(self.tr("Instellingen", "Settings"));
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.label(self.tr("Taal", "Language"));
            let mut selected = self.language_preference;
            let option_label = |lang: LanguagePreference| match (self.language, lang) {
                (crate::i18n::Language::Dutch, LanguagePreference::System) => {
                    "Systeem (automatisch)"
                }
                (crate::i18n::Language::English, LanguagePreference::System) => "System (auto)",
                (crate::i18n::Language::Dutch, LanguagePreference::Dutch) => "Nederlands",
                (crate::i18n::Language::English, LanguagePreference::Dutch) => "Dutch",
                (crate::i18n::Language::Dutch, LanguagePreference::English) => "Engels",
                (crate::i18n::Language::English, LanguagePreference::English) => "English",
            };
            egui::ComboBox::from_id_salt("language-select")
                .selected_text(option_label(selected))
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut selected,
                        LanguagePreference::System,
                        option_label(LanguagePreference::System),
                    );
                    ui.selectable_value(
                        &mut selected,
                        LanguagePreference::Dutch,
                        option_label(LanguagePreference::Dutch),
                    );
                    ui.selectable_value(
                        &mut selected,
                        LanguagePreference::English,
                        option_label(LanguagePreference::English),
                    );
                });
            if selected != self.language_preference {
                self.update_language_preference(selected);
                self.status = self.tr("Taal gewijzigd.", "Language updated.").to_string();
            }
        });
        ui.add_space(12.0);
        ui.horizontal(|ui| {
            let threshold_label = self.tr("Onzekerheidsdrempel", "Uncertainty threshold");
            let slider = egui::Slider::new(&mut self.pending_presence_threshold, 0.0..=1.0)
                .text(threshold_label)
                .custom_formatter(|v, _| format!("{:.0}%", v * 100.0));
            ui.add(slider);
            if ui.button(self.tr("Herbereken", "Recompute")).clicked() {
                self.presence_threshold = self.pending_presence_threshold;
                self.apply_presence_threshold();
                self.status = format!(
                    "{}: {:.0}%",
                    self.tr(
                        "Onzekerheidsdrempel toegepast",
                        "Uncertainty threshold applied"
                    ),
                    self.presence_threshold * 100.0
                );
                self.panel = Panel::Results;
            }
        });
        ui.add_space(12.0);
        ui.horizontal(|ui| {
            ui.label(self.tr("Batchgrootte", "Batch size"));
            let resp = ui.add(
                egui::DragValue::new(&mut self.batch_size)
                    .range(1..=64)
                    .speed(1),
            );
            if resp.changed() {
                self.status = self
                    .tr(
                        "Nieuwe batchgrootte wordt toegepast bij volgende scan",
                        "New batch size will be applied on the next scan",
                    )
                    .to_string();
            }
        });
        ui.add_space(12.0);
        ui.horizontal(|ui| {
            ui.label(self.tr("Achtergrondlabels", "Background labels"));
            let response = ui.text_edit_singleline(&mut self.background_labels_input);
            if response.changed() {
                self.sync_background_labels();
                self.status = self
                    .tr(
                        "Achtergrondlabels bijgewerkt voor huidige resultaten",
                        "Background labels updated for current results",
                    )
                    .to_string();
            }
        });

        ui.add_space(12.0);
        ui.separator();
        ui.add_space(6.0);
        let improve_label = self.tr(
            "Help de herkenning te verbeteren",
            "Help improve recognition",
        );
        ui.checkbox(&mut self.improve_recognition, improve_label);
        ui.label(
            self.tr(
                "Wanneer je handmatig een categorie wijzigt, uploaden we die afbeeldingen op de achtergrond naar Roboflow.",
                "When you manually change a category, we upload those images to Roboflow in the background.",
            ),
        );
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.label(self.tr(
                "Roboflow dataset (bijv. voederhuiscamera)",
                "Roboflow dataset (e.g. voederhuiscamera)",
            ));
            ui.text_edit_singleline(&mut self.roboflow_dataset_input);
        });
        ui.add_space(4.0);
        ui.label(self.tr(
            "Uploads gebruiken een ingebouwde Roboflow API-sleutel en draaien volledig op de achtergrond.",
            "Uploads use an embedded Roboflow API key and run fully in the background.",
        ));

        ui.add_space(16.0);
        ui.separator();
        ui.add_space(6.0);
        ui.heading(self.tr("Versies", "Versions"));
        ui.label(format!(
            "{}: {}",
            self.tr("App versie", "App version"),
            self.app_version
        ));
        ui.label(format!(
            "{}: {}",
            self.tr(
                "Herkenningsmodel en soortenlijstversie",
                "Model and species list version"
            ),
            self.model_version
        ));
        self.render_update_section(ui);
    }
}
