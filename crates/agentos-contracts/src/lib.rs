use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeSnapshotDto {
    pub revision: u64,
    pub current_surface_id: String,
    pub breadcrumb: Vec<BreadcrumbItemDto>,
    pub surface: SurfaceDto,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BreadcrumbItemDto {
    pub surface_id: String,
    pub title: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SurfaceDto {
    pub id: String,
    pub title: String,
    pub icon: String,
    pub elements: Vec<ElementDto>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ElementDto {
    TimePanel {
        id: String,
        title: String,
        timezone: String,
        column: u8,
    },
    AppIcon {
        id: String,
        title: String,
        icon: String,
        status: String,
        target_surface_id: String,
        column: u8,
    },
    TextPanel {
        id: String,
        title: String,
        runs: Vec<TextRunDto>,
        column: u8,
    },
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum TextRunDto {
    Text {
        content: String,
    },
    SurfaceLink {
        label: String,
        target_surface_id: String,
    },
}
