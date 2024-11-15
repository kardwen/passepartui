use icu::{
    calendar::{DateTime, Gregorian},
    datetime::{options::length, TypedDateTimeFormatter},
    locid::locale,
};
use std::{fs::Metadata, path::Path, time::UNIX_EPOCH};

#[derive(Debug)]
pub struct PasswordInfo {
    pub pass_id: String,
    pub metadata: Metadata,
}

impl PasswordInfo {
    pub fn new(relative_path: &Path, metadata: Metadata) -> Self {
        PasswordInfo {
            pass_id: Self::build_pass_id(relative_path),
            metadata,
        }
    }

    fn build_pass_id(relative_path: &Path) -> String {
        let parent = relative_path
            .parent()
            .expect("yields None when passed an empty string");
        let file_stem = relative_path.file_stem().expect("No file name");
        parent
            .join(Path::new(file_stem))
            .to_str()
            .expect("Unicode conversion failed")
            .to_string()
    }

    pub fn last_modified(&self) -> String {
        if let Ok(modified_system_time) = self.metadata.modified() {
            if let Ok(duration) = modified_system_time.duration_since(UNIX_EPOCH) {
                // TypedDateTimeFormatter
                let options =
                    length::Bag::from_date_time_style(length::Date::Medium, length::Time::Short)
                        .into();
                let dtf =
                    TypedDateTimeFormatter::<Gregorian>::try_new(&locale!("en").into(), options)
                        .expect("Failed to create TypedDateTimeFormatter instance.");
                // DateTime
                let datetime =
                    DateTime::from_minutes_since_local_unix_epoch(duration.as_secs() as i32 / 60)
                        .to_calendar(Gregorian);
                return dtf.format_to_string(&datetime);
            }
        }
        String::from("Unknown")
    }

    pub fn pass_id(&self) -> String {
        self.pass_id.clone()
    }
}
