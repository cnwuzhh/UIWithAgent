use agentos_contracts::{BreadcrumbItemDto, ElementDto, RuntimeSnapshotDto, SurfaceDto};
use agentos_domain::{AppIcon, Element, GuiDocument, Surface, SurfaceId, TimePanel};

pub struct Runtime {
    document: GuiDocument,
    current_surface_id: SurfaceId,
}

impl Runtime {
    pub fn demo() -> Self {
        let desktop_id = surface_id("surface-desktop");
        let work_id = surface_id("surface-work");
        let document = GuiDocument::new(
            1,
            desktop_id.clone(),
            vec![
                Surface {
                    id: desktop_id.clone(),
                    parent_id: None,
                    title: "桌面".into(),
                    icon: "layout-dashboard".into(),
                    elements: vec![
                        Element::TimePanel(TimePanel {
                            id: "time-local".into(),
                            title: "当地时间".into(),
                            timezone: "local".into(),
                            column: 1,
                        }),
                        Element::TimePanel(TimePanel {
                            id: "time-london".into(),
                            title: "伦敦".into(),
                            timezone: "Europe/London".into(),
                            column: 5,
                        }),
                        Element::TimePanel(TimePanel {
                            id: "time-los-angeles".into(),
                            title: "洛杉矶".into(),
                            timezone: "America/Los_Angeles".into(),
                            column: 9,
                        }),
                        Element::AppIcon(AppIcon {
                            id: "app-workspace".into(),
                            title: "工作桌面".into(),
                            target_surface_id: work_id.clone(),
                            column: 1,
                        }),
                    ],
                },
                Surface {
                    id: work_id,
                    parent_id: Some(desktop_id.clone()),
                    title: "工作桌面".into(),
                    icon: "briefcase-business".into(),
                    elements: vec![Element::TimePanel(TimePanel {
                        id: "time-work".into(),
                        title: "工作时间".into(),
                        timezone: "local".into(),
                        column: 1,
                    })],
                },
            ],
        )
        .expect("demo document must be valid");

        Self {
            document,
            current_surface_id: desktop_id,
        }
    }

    pub fn snapshot(&self) -> RuntimeSnapshotDto {
        self.snapshot_for(&self.current_surface_id)
            .expect("current surface must exist")
    }

    pub fn open_surface(&mut self, surface_id: &str) -> Result<RuntimeSnapshotDto, String> {
        let target = SurfaceId::new(surface_id).map_err(|error| error.to_string())?;
        let snapshot = self.snapshot_for(&target)?;
        self.current_surface_id = target;
        Ok(snapshot)
    }

    fn snapshot_for(&self, surface_id: &SurfaceId) -> Result<RuntimeSnapshotDto, String> {
        let surface = self
            .document
            .surface(surface_id)
            .map_err(|error| error.to_string())?;
        let breadcrumb = self
            .document
            .path_to(surface_id)
            .map_err(|error| error.to_string())?
            .into_iter()
            .map(|item| BreadcrumbItemDto {
                surface_id: item.id.as_str().into(),
                title: item.title.clone(),
            })
            .collect();

        Ok(RuntimeSnapshotDto {
            revision: self.document.revision(),
            current_surface_id: surface.id.as_str().into(),
            breadcrumb,
            surface: self.surface_to_dto(surface)?,
        })
    }

    fn surface_to_dto(&self, surface: &Surface) -> Result<SurfaceDto, String> {
        Ok(SurfaceDto {
            id: surface.id.as_str().into(),
            title: surface.title.clone(),
            icon: surface.icon.clone(),
            elements: surface
                .elements
                .iter()
                .map(|element| self.element_to_dto(element))
                .collect::<Result<Vec<_>, _>>()?,
        })
    }

    fn element_to_dto(&self, element: &Element) -> Result<ElementDto, String> {
        Ok(match element {
            Element::TimePanel(panel) => ElementDto::TimePanel {
                id: panel.id.clone(),
                title: panel.title.clone(),
                timezone: panel.timezone.clone(),
                column: panel.column,
            },
            Element::AppIcon(icon) => {
                let target = self
                    .document
                    .surface(&icon.target_surface_id)
                    .map_err(|error| error.to_string())?;
                ElementDto::AppIcon {
                    id: icon.id.clone(),
                    title: icon.title.clone(),
                    icon: target.icon.clone(),
                    status: "ready".into(),
                    target_surface_id: icon.target_surface_id.as_str().into(),
                    column: icon.column,
                }
            }
        })
    }
}

fn surface_id(value: &str) -> SurfaceId {
    SurfaceId::new(value).expect("static surface id must be valid")
}

#[cfg(test)]
mod tests {
    use super::Runtime;

    #[test]
    fn initial_snapshot_has_three_clocks() {
        let snapshot = Runtime::demo().snapshot();
        let clock_count = snapshot
            .surface
            .elements
            .iter()
            .filter(|element| matches!(element, agentos_contracts::ElementDto::TimePanel { .. }))
            .count();

        assert_eq!(clock_count, 3);
        assert_eq!(snapshot.revision, 1);
    }

    #[test]
    fn opening_surface_updates_real_tree_path() {
        let mut runtime = Runtime::demo();
        let snapshot = runtime.open_surface("surface-work").unwrap();

        assert_eq!(snapshot.current_surface_id, "surface-work");
        assert_eq!(snapshot.breadcrumb.len(), 2);
        assert_eq!(snapshot.breadcrumb[0].title, "桌面");
        assert_eq!(snapshot.breadcrumb[1].title, "工作桌面");
    }

    #[test]
    fn surface_entry_inherits_target_icon() {
        let snapshot = Runtime::demo().snapshot();
        let icon = snapshot
            .surface
            .elements
            .iter()
            .find_map(|element| match element {
                agentos_contracts::ElementDto::AppIcon { icon, .. } => Some(icon),
                _ => None,
            })
            .unwrap();

        assert_eq!(icon, "briefcase-business");
    }
}
