#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub struct State {
    pub main: MainState,
    pub search: SearchState,
    pub overlay: OverlayState,
}

#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub enum MainState {
    Table,
    #[default]
    Preview,
    Secrets,
}

#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub enum SearchState {
    #[default]
    Inactive,
    Suspended,
    Active,
}

#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub enum OverlayState {
    #[default]
    Inactive,
    Help,
    File,
}
