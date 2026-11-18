use super::*;
use crate::events::RecordingSink;

struct Setup {
    _dir: tempfile::TempDir,
    path: PathBuf,
    sink: Arc<RecordingSink>,
}

impl Setup {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        // 設定ディレクトリがまだない状態から始める
        let path = dir
            .path()
            .join("com.rinontech.esp-latch")
            .join(SETTINGS_FILE);
        Self {
            _dir: dir,
            path,
            sink: Arc::new(RecordingSink::default()),
        }
    }

    fn write(&self, text: &str) {
        std::fs::create_dir_all(self.path.parent().unwrap()).unwrap();
        std::fs::write(&self.path, text).unwrap();
    }

    fn store(&self) -> SettingsStore {
        SettingsStore::load(self.path.clone(), self.sink.clone())
    }

    fn saved(&self) -> Value {
        serde_json::from_str(&std::fs::read_to_string(&self.path).unwrap()).unwrap()
    }
}

#[test]
fn defaults_match_the_spec() {
    let d = Settings::default();
    assert_eq!(d.language, Language::System);
    assert_eq!(d.theme, Theme::System);
    assert_eq!(d.default_filter, DefaultFilter::All);
    assert!(d.open_finder_after_mount);
    assert!(d.auto_refresh);
    assert!(d.confirm_eject);
    assert!(!d.auto_empty_trash_on_unmount);
    assert!(d.show_command);
}

#[test]
fn missing_file_uses_defaults_without_creating_it() {
    let s = Setup::new();
    let store = s.store();
    assert_eq!(store.get(), Settings::default());
    assert!(!s.path.exists());
    assert!(!s.path.parent().unwrap().exists());
}

#[test]
fn first_change_creates_the_file_and_emits() {
    let s = Setup::new();
    let store = s.store();
    let patch = SettingsPatch {
        language: Some(Language::En),
        ..SettingsPatch::default()
    };
    let updated = store.update(&patch).unwrap();
    assert_eq!(updated.language, Language::En);
    assert_eq!(store.get(), updated);
    assert_eq!(
        s.saved(),
        serde_json::json!({
            "language": "en",
            "theme": "system",
            "default_filter": "all",
            "open_finder_after_mount": true,
            "auto_refresh": true,
            "confirm_eject": true,
            "auto_empty_trash_on_unmount": false,
            "show_command": true,
        })
    );
    assert_eq!(s.sink.take(), [AppEvent::SettingsChanged(updated)]);
    // 一時ファイルを残さない
    let files: Vec<_> = std::fs::read_dir(s.path.parent().unwrap())
        .unwrap()
        .collect();
    assert_eq!(files.len(), 1);
}

#[test]
fn same_value_is_not_written() {
    let s = Setup::new();
    let store = s.store();
    let patch = SettingsPatch {
        auto_refresh: Some(true),
        ..SettingsPatch::default()
    };
    assert_eq!(store.update(&patch).unwrap(), Settings::default());
    assert!(!s.path.exists());
    assert!(s.sink.take().is_empty());
}

#[test]
fn saved_settings_are_read_on_next_start() {
    let s = Setup::new();
    s.store()
        .update(&SettingsPatch {
            language: Some(Language::En),
            auto_empty_trash_on_unmount: Some(true),
            default_filter: Some(DefaultFilter::Esp),
            ..SettingsPatch::default()
        })
        .unwrap();
    let next = s.store().get();
    assert_eq!(next.language, Language::En);
    assert!(next.auto_empty_trash_on_unmount);
    assert_eq!(next.default_filter, DefaultFilter::Esp);
    assert!(next.auto_refresh);
}

#[test]
fn broken_file_falls_back_to_defaults_and_is_overwritten() {
    let s = Setup::new();
    s.write("{ this is not json");
    let store = s.store();
    assert_eq!(store.get(), Settings::default());
    store
        .update(&SettingsPatch {
            theme: Some(Theme::Dark),
            ..SettingsPatch::default()
        })
        .unwrap();
    assert_eq!(s.saved()["theme"], "dark");
    assert_eq!(s.saved()["language"], "system");
}

#[test]
fn unknown_or_wrongly_typed_values_reset_only_that_item() {
    let s = Setup::new();
    s.write(
        r#"{
            "language": "fr",
            "theme": "dark",
            "default_filter": 3,
            "auto_refresh": false,
            "confirm_eject": "yes",
            "future_option": 1
        }"#,
    );
    let settings = s.store().get();
    assert_eq!(settings.language, Language::System);
    assert_eq!(settings.theme, Theme::Dark);
    assert_eq!(settings.default_filter, DefaultFilter::All);
    assert!(!settings.auto_refresh);
    assert!(settings.confirm_eject);
    // ない項目は既定値で補う
    assert!(settings.show_command);
}

#[test]
fn non_object_json_falls_back_to_defaults() {
    let s = Setup::new();
    s.write("[1, 2, 3]");
    assert_eq!(s.store().get(), Settings::default());
}

#[test]
fn zh_hans_round_trips() {
    let s = Setup::new();
    s.store()
        .update(&SettingsPatch {
            language: Some(Language::ZhHans),
            ..SettingsPatch::default()
        })
        .unwrap();
    assert_eq!(s.saved()["language"], "zh-Hans");
    assert_eq!(s.store().get().language, Language::ZhHans);
}

#[test]
fn patch_rejects_unknown_fields_and_values() {
    let patch: SettingsPatch = serde_json::from_str(r#"{"theme":"light"}"#).unwrap();
    assert_eq!(patch.theme, Some(Theme::Light));
    assert!(serde_json::from_str::<SettingsPatch>(r#"{"volume":1}"#).is_err());
    assert!(serde_json::from_str::<SettingsPatch>(r#"{"theme":"blue"}"#).is_err());
}

#[test]
fn write_failure_keeps_the_previous_value() {
    let s = Setup::new();
    // 設定ディレクトリの場所にファイルを置き、書き込めなくする
    std::fs::write(s.path.parent().unwrap(), "").unwrap();
    let store = s.store();
    let result = store.update(&SettingsPatch {
        theme: Some(Theme::Light),
        ..SettingsPatch::default()
    });
    assert!(result.is_err());
    assert_eq!(store.get(), Settings::default());
    assert!(s.sink.take().is_empty());
}

#[test]
fn concurrent_updates_are_applied_in_turn() {
    let s = Setup::new();
    let store = Arc::new(s.store());
    let handles: Vec<_> = [
        SettingsPatch {
            theme: Some(Theme::Dark),
            ..SettingsPatch::default()
        },
        SettingsPatch {
            show_command: Some(false),
            ..SettingsPatch::default()
        },
    ]
    .into_iter()
    .map(|patch| {
        let store = Arc::clone(&store);
        std::thread::spawn(move || store.update(&patch).unwrap())
    })
    .collect();
    for handle in handles {
        handle.join().unwrap();
    }
    // どちらの変更も失われない
    let saved = s.saved();
    assert_eq!(saved["theme"], "dark");
    assert_eq!(saved["show_command"], false);
    // 最後のイベントが、最終の値を配る
    let events = s.sink.take();
    assert_eq!(events.len(), 2);
    assert_eq!(events[1], AppEvent::SettingsChanged(store.get()));
}
