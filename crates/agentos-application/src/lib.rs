use agentos_contracts::{BreadcrumbItemDto, ElementDto, RuntimeSnapshotDto, SurfaceDto};

pub fn initial_runtime_snapshot() -> RuntimeSnapshotDto {
    RuntimeSnapshotDto {
        revision: 1,
        current_surface_id: "surface-desktop".into(),
        breadcrumb: vec![BreadcrumbItemDto {
            surface_id: "surface-desktop".into(),
            title: "桌面".into(),
        }],
        surface: SurfaceDto {
            id: "surface-desktop".into(),
            title: "桌面".into(),
            icon: "layout-dashboard".into(),
            elements: vec![
                ElementDto::TimePanel {
                    id: "time-local".into(),
                    title: "当地时间".into(),
                    timezone: "local".into(),
                    column: 1,
                },
                ElementDto::TimePanel {
                    id: "time-london".into(),
                    title: "伦敦".into(),
                    timezone: "Europe/London".into(),
                    column: 5,
                },
                ElementDto::TimePanel {
                    id: "time-los-angeles".into(),
                    title: "洛杉矶".into(),
                    timezone: "America/Los_Angeles".into(),
                    column: 9,
                },
                ElementDto::AppIcon {
                    id: "app-workspace".into(),
                    title: "工作桌面".into(),
                    icon: "briefcase-business".into(),
                    status: "ready".into(),
                    column: 1,
                },
            ],
        },
    }
}

#[cfg(test)]
mod tests {
    use super::initial_runtime_snapshot;

    #[test]
    fn initial_snapshot_has_three_clocks() {
        let snapshot = initial_runtime_snapshot();
        let clock_count = snapshot
            .surface
            .elements
            .iter()
            .filter(|element| matches!(element, agentos_contracts::ElementDto::TimePanel { .. }))
            .count();

        assert_eq!(clock_count, 3);
        assert_eq!(snapshot.revision, 1);
    }
}
