#[derive(Debug)]
pub enum PasswordEvent {
    Status(Result<Option<String>, passepartout::Error>),
    PasswordFile {
        pass_id: String,
        file_contents: String,
    },
    OneTimePassword {
        pass_id: String,
        otp: String,
    },
}
