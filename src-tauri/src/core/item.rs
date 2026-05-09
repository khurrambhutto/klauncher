use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LauncherItem {
    pub id: String,
    pub title: String,
    pub subtitle: Option<String>,
    pub icon: Option<String>,
    pub kind: ItemKind,
}

#[derive(Clone, Debug, Serialize)]
#[allow(dead_code)]
#[serde(rename_all = "camelCase")]
pub enum ItemKind {
    App,
    File,
    Folder,
    Text,
    Clipboard,
    Script,
}
