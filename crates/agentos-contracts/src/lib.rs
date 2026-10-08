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
pub struct BuildSubmissionDto {
    pub message: String,
    pub snapshot: RuntimeSnapshotDto,
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
    pub columns: u8,
    pub rows: u8,
    pub elements: Vec<ElementDto>,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GridRectDto {
    pub x: u8,
    pub y: u8,
    pub width: u8,
    pub height: u8,
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
        rect: GridRectDto,
    },
    AppIcon {
        id: String,
        title: String,
        icon: String,
        status: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        target_surface_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        build_task_id: Option<String>,
        rect: GridRectDto,
    },
    TextPanel {
        id: String,
        title: String,
        runs: Vec<TextRunDto>,
        rect: GridRectDto,
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
    use super::{ElementDto, GridRectDto, TextRunDto};
    use serde_json::json;

    #[test]
    fn surface_targets_are_serialized_as_camel_case() {
        let app_icon = ElementDto::AppIcon {
            id: "work".into(),
            title: "Work".into(),
            icon: "briefcase".into(),
            status: "ready".into(),
            target_surface_id: Some("surface-work".into()),
            build_task_id: None,
            rect: GridRectDto {
                x: 0,
                y: 1,
                width: 2,
                height: 1,
            },
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
                "rect": { "x": 0, "y": 1, "width": 2, "height": 1 }
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
