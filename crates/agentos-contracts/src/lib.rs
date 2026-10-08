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
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
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
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum TextRunDto {
    Text {
        content: String,
    },
    SurfaceLink {
        label: String,
        target_surface_id: String,
    },
}

#[cfg(test)]
mod tests {
    use super::{ElementDto, TextRunDto};
    use serde_json::json;

    #[test]
    fn surface_targets_are_serialized_as_camel_case() {
        let app_icon = ElementDto::AppIcon {
            id: "work".into(),
            title: "Work".into(),
            icon: "briefcase".into(),
            status: "ready".into(),
            target_surface_id: "surface-work".into(),
            column: 1,
        };
        let surface_link = TextRunDto::SurfaceLink {
            label: "Work".into(),
            target_surface_id: "surface-work".into(),
        };

        assert_eq!(
            serde_json::to_value(app_icon).unwrap(),
            json!({
                "type": "appIcon",
                "id": "work",
                "title": "Work",
                "icon": "briefcase",
                "status": "ready",
                "targetSurfaceId": "surface-work",
                "column": 1
            })
        );
        assert_eq!(
            serde_json::to_value(surface_link).unwrap(),
            json!({
                "type": "surfaceLink",
                "label": "Work",
                "targetSurfaceId": "surface-work"
            })
        );
    }
}
