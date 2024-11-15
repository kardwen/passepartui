#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    Navigation(NavigationAction),
    Password(PasswordAction),
    Search(SearchAction),
    DisplayStatus(String),
    ResetStatus,
    DisplaySecrets {
        pass_id: String,
        file_contents: String,
    },
    DisplayOneTimePassword {
        pass_id: String,
        one_time_password: String,
    },
    NoOp,
}

#[derive(Debug, Clone, PartialEq)]
pub enum NavigationAction {
    Back,
    Next,
    Leave,
    Down,
    Up,
    PageDown,
    PageUp,
    Top,
    Bottom,
    Preview,
    Secrets,
    Search,
    Help,
    File,
    Select(usize),
    SelectAndFetch(usize),
    Quit,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SearchAction {
    Insert(char),
    RemoveLeft,
    RemoveRight,
    MoveLeft,
    MoveRight,
    MoveToStart,
    MoveToEnd,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PasswordAction {
    CopyPassId,
    Fetch,
    CopyPassword,
    CopyLogin,
    FetchOneTimePassword,
    CopyOneTimePassword,
}
