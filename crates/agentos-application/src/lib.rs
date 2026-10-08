use agentos_contracts::{
    BreadcrumbItemDto, BuildSubmissionDto, ElementDto, GridRectDto, RuntimeSnapshotDto, SurfaceDto,
    TextRunDto,
};
use agentos_domain::{
    AppIcon, AppIconStatus, Element, GridRect, GuiDocument, GuiOperation, OperationTransaction,
    Surface, SurfaceId, TextPanel, TextRun, TimePanel,
};
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct BuildTask {
    pub id: String,
    pub surface_id: SurfaceId,
    pub request: String,
    pub placeholder_element_id: String,
}

pub struct Runtime {
    document: GuiDocument,
    current_surface_id: SurfaceId,
    build_tasks: HashMap<String, BuildTask>,
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
                    columns: 12,
                    rows: 4,
                    elements: vec![
                        Element::TimePanel(TimePanel {
                            id: "time-local".into(),
                            title: "当地时间".into(),
                            timezone: "local".into(),
                            rect: rect(0, 0, 4, 1),
                        }),
                        Element::TimePanel(TimePanel {
                            id: "time-london".into(),
                            title: "伦敦".into(),
                            timezone: "Europe/London".into(),
                            rect: rect(4, 0, 4, 1),
                        }),
                        Element::TimePanel(TimePanel {
                            id: "time-los-angeles".into(),
                            title: "洛杉矶".into(),
                            timezone: "America/Los_Angeles".into(),
                            rect: rect(8, 0, 4, 1),
                        }),
                        Element::AppIcon(AppIcon {
                            id: "app-workspace".into(),
                            title: "工作桌面".into(),
                            status: AppIconStatus::Ready,
                            target_surface_id: Some(work_id.clone()),
                            build_task_id: None,
                            rect: rect(0, 1, 2, 1),
                        }),
                        Element::TextPanel(TextPanel {
                            id: "text-getting-started".into(),
                            title: "开始使用".into(),
                            runs: vec![
                                TextRun::Text("项目资料和工作时钟位于 ".into()),
                                TextRun::SurfaceLink {
                                    label: "工作桌面".into(),
                                    target_surface_id: work_id.clone(),
                                },
                                TextRun::Text("。正文中的引用不会改变 Surface 主树关系。".into()),
                            ],
                            rect: rect(2, 1, 6, 1),
                        }),
                    ],
                },
                Surface {
                    id: work_id,
                    parent_id: Some(desktop_id.clone()),
                    title: "工作桌面".into(),
                    icon: "briefcase-business".into(),
                    columns: 12,
                    rows: 4,
                    elements: vec![Element::TimePanel(TimePanel {
                        id: "time-work".into(),
                        title: "工作时间".into(),
                        timezone: "local".into(),
                        rect: rect(0, 0, 4, 1),
                    })],
                },
            ],
        )
        .expect("demo document must be valid");

        Self {
            document,
            current_surface_id: desktop_id,
            build_tasks: HashMap::new(),
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

    pub fn reposition_element(
        &mut self,
        surface_id: &str,
        element_id: &str,
        x: u8,
        y: u8,
    ) -> Result<RuntimeSnapshotDto, String> {
        let surface_id = SurfaceId::new(surface_id).map_err(|error| error.to_string())?;
        self.document = self
            .document
            .reposition_element(&surface_id, element_id, x, y)
            .map_err(|error| error.to_string())?;
        self.snapshot_for(&self.current_surface_id)
    }

    pub fn resize_element(
        &mut self,
        surface_id: &str,
        element_id: &str,
        width: u8,
        height: u8,
    ) -> Result<RuntimeSnapshotDto, String> {
        let surface_id = SurfaceId::new(surface_id).map_err(|error| error.to_string())?;
        self.document = self
            .document
            .resize_element(&surface_id, element_id, width, height)
            .map_err(|error| error.to_string())?;
        self.snapshot_for(&self.current_surface_id)
    }

    pub fn add_time_panel(
        &mut self,
        surface_id: &str,
        title: &str,
        timezone: &str,
    ) -> Result<RuntimeSnapshotDto, String> {
        let surface_id = SurfaceId::new(surface_id).map_err(|error| error.to_string())?;
        let element = Element::TimePanel(TimePanel {
            id: format!("time-created-{}", self.document.revision()),
            title: title.trim().into(),
            timezone: timezone.trim().into(),
            rect: rect(0, 0, 4, 1),
        });
        self.document = self
            .document
            .apply_transaction(&OperationTransaction {
                operations: vec![GuiOperation::AddElementAuto {
                    surface_id,
                    element,
                }],
            })
            .map_err(|error| error.to_string())?
            .document;
        self.snapshot_for(&self.current_surface_id)
    }

    pub fn remove_element(
        &mut self,
        surface_id: &str,
        element_id: &str,
    ) -> Result<RuntimeSnapshotDto, String> {
        let surface_id = SurfaceId::new(surface_id).map_err(|error| error.to_string())?;
        let is_build_placeholder = self
            .document
            .surface(&surface_id)
            .map_err(|error| error.to_string())?
            .elements
            .iter()
            .any(|element| {
                matches!(
                    element,
                    Element::AppIcon(icon)
                        if icon.id == element_id && icon.status == AppIconStatus::Building
                )
            });
        if is_build_placeholder {
            return Err("building placeholder must be removed by cancelling its task".into());
        }
        self.document = self
            .document
            .apply_transaction(&OperationTransaction {
                operations: vec![GuiOperation::RemoveElement {
                    surface_id,
                    element_id: element_id.into(),
                }],
            })
            .map_err(|error| error.to_string())?
            .document;
        self.snapshot_for(&self.current_surface_id)
    }

    pub fn submit_build(
        &mut self,
        surface_id: &str,
        request: &str,
    ) -> Result<BuildSubmissionDto, String> {
        let request = request.trim();
        if request.is_empty() {
            return Err("build request cannot be empty".into());
        }

        let surface_id = SurfaceId::new(surface_id).map_err(|error| error.to_string())?;
        if surface_id != self.current_surface_id {
            return Err("build request surface must be the current surface".into());
        }
        let build_task_id = format!("build-{}", self.document.revision());
        let placeholder_element_id = format!("build-placeholder-{}", self.document.revision());
        let outcome = self
            .document
            .apply_transaction(&OperationTransaction {
                operations: vec![GuiOperation::AddElementAuto {
                    surface_id: surface_id.clone(),
                    element: Element::AppIcon(AppIcon {
                        id: placeholder_element_id.clone(),
                        title: "新应用 · 构建中".into(),
                        status: AppIconStatus::Building,
                        target_surface_id: None,
                        build_task_id: Some(build_task_id.clone()),
                        rect: rect(0, 0, 2, 1),
                    }),
                }],
            })
            .map_err(|error| error.to_string())?;

        self.document = outcome.document;
        self.build_tasks.insert(
            build_task_id.clone(),
            BuildTask {
                id: build_task_id,
                surface_id,
                request: request.into(),
                placeholder_element_id,
            },
        );

        Ok(BuildSubmissionDto {
            message: "收到".into(),
            snapshot: self.snapshot_for(&self.current_surface_id)?,
        })
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
            columns: surface.columns,
            rows: surface.rows,
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
                rect: rect_to_dto(panel.rect),
            },
            Element::AppIcon(icon) => {
                let target = icon
                    .target_surface_id
                    .as_ref()
                    .map(|target_surface_id| self.document.surface(target_surface_id))
                    .transpose()
                    .map_err(|error| error.to_string())?;
                ElementDto::AppIcon {
                    id: icon.id.clone(),
                    title: icon.title.clone(),
                    icon: target.map_or_else(|| "hammer".into(), |surface| surface.icon.clone()),
                    status: match icon.status {
                        AppIconStatus::Ready => "ready",
                        AppIconStatus::Building => "building",
                    }
                    .into(),
                    target_surface_id: icon.target_surface_id.as_ref().map(|id| id.as_str().into()),
                    build_task_id: icon.build_task_id.clone(),
                    rect: rect_to_dto(icon.rect),
                }
            }
            Element::TextPanel(panel) => ElementDto::TextPanel {
                id: panel.id.clone(),
                title: panel.title.clone(),
                runs: panel
                    .runs
                    .iter()
                    .map(|run| match run {
                        TextRun::Text(content) => TextRunDto::Text {
                            content: content.clone(),
                        },
                        TextRun::SurfaceLink {
                            label,
                            target_surface_id,
                        } => TextRunDto::SurfaceLink {
                            label: label.clone(),
                            target_surface_id: target_surface_id.as_str().into(),
                        },
                    })
                    .collect(),
                rect: rect_to_dto(panel.rect),
            },
        })
    }
}

fn surface_id(value: &str) -> SurfaceId {
    SurfaceId::new(value).expect("static surface id must be valid")
}

fn rect(x: u8, y: u8, width: u8, height: u8) -> GridRect {
    GridRect::new(x, y, width, height).expect("static rect must be valid")
}

fn rect_to_dto(rect: GridRect) -> GridRectDto {
    GridRectDto {
        x: rect.x,
        y: rect.y,
        width: rect.width,
        height: rect.height,
    }
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

    #[test]
    fn embedded_text_exposes_surface_target() {
        let snapshot = Runtime::demo().snapshot();
        let target = snapshot
            .surface
            .elements
            .iter()
            .find_map(|element| match element {
                agentos_contracts::ElementDto::TextPanel { runs, .. } => {
                    runs.iter().find_map(|run| match run {
                        agentos_contracts::TextRunDto::SurfaceLink {
                            target_surface_id, ..
                        } => Some(target_surface_id),
                        _ => None,
                    })
                }
                _ => None,
            })
            .unwrap();

        assert_eq!(target, "surface-work");
    }

    #[test]
    fn reposition_returns_incremented_snapshot() {
        let mut runtime = Runtime::demo();
        let snapshot = runtime
            .reposition_element("surface-desktop", "app-workspace", 8, 2)
            .unwrap();

        assert_eq!(snapshot.revision, 2);
        let rect = snapshot
            .surface
            .elements
            .iter()
            .find_map(|element| match element {
                agentos_contracts::ElementDto::AppIcon { id, rect, .. }
                    if id == "app-workspace" =>
                {
                    Some(rect)
                }
                _ => None,
            })
            .unwrap();
        assert_eq!((rect.x, rect.y), (8, 2));
    }

    #[test]
    fn resize_returns_incremented_snapshot() {
        let mut runtime = Runtime::demo();
        let snapshot = runtime
            .resize_element("surface-desktop", "app-workspace", 2, 2)
            .unwrap();

        assert_eq!(snapshot.revision, 2);
        let rect = snapshot
            .surface
            .elements
            .iter()
            .find_map(|element| match element {
                agentos_contracts::ElementDto::AppIcon { id, rect, .. }
                    if id == "app-workspace" =>
                {
                    Some(rect)
                }
                _ => None,
            })
            .unwrap();
        assert_eq!((rect.width, rect.height), (2, 2));
    }

    #[test]
    fn add_and_remove_time_panel_return_authoritative_snapshots() {
        let mut runtime = Runtime::demo();
        let added = runtime
            .add_time_panel("surface-desktop", "东京", "Asia/Tokyo")
            .unwrap();
        let id = added
            .surface
            .elements
            .iter()
            .find_map(|element| match element {
                agentos_contracts::ElementDto::TimePanel { id, title, .. } if title == "东京" => {
                    Some(id.clone())
                }
                _ => None,
            })
            .unwrap();

        assert_eq!(added.revision, 2);
        let removed = runtime.remove_element("surface-desktop", &id).unwrap();
        assert_eq!(removed.revision, 3);
        assert!(
            !removed
                .surface
                .elements
                .iter()
                .any(|element| match element {
                    agentos_contracts::ElementDto::TimePanel { id: current, .. } => current == &id,
                    _ => false,
                })
        );
    }

    #[test]
    fn submit_build_records_task_and_adds_building_placeholder() {
        let mut runtime = Runtime::demo();
        let submission = runtime
            .submit_build("surface-desktop", "在当地时间上增加天气")
            .unwrap();

        assert_eq!(submission.message, "收到");
        assert_eq!(submission.snapshot.revision, 2);
        assert_eq!(runtime.build_tasks.len(), 1);
        let task = runtime.build_tasks.values().next().unwrap();
        assert_eq!(task.id, "build-1");
        assert_eq!(task.surface_id.as_str(), "surface-desktop");
        assert_eq!(task.request, "在当地时间上增加天气");
        assert_eq!(task.placeholder_element_id, "build-placeholder-1");
        assert!(submission.snapshot.surface.elements.iter().any(|element| {
            matches!(
                element,
                agentos_contracts::ElementDto::AppIcon {
                    status,
                    build_task_id: Some(task_id),
                    target_surface_id: None,
                    ..
                } if status == "building" && task_id == "build-1"
            )
        }));
    }

    #[test]
    fn submit_build_rejects_empty_request_without_mutating_runtime() {
        let mut runtime = Runtime::demo();

        assert!(runtime.submit_build("surface-desktop", "  ").is_err());
        assert_eq!(runtime.snapshot().revision, 1);
        assert!(runtime.build_tasks.is_empty());
    }

    #[test]
    fn submit_build_rejects_non_current_surface_without_mutating_runtime() {
        let mut runtime = Runtime::demo();

        assert!(
            runtime
                .submit_build("surface-work", "创建天气应用")
                .is_err()
        );
        assert_eq!(runtime.snapshot().revision, 1);
        assert!(runtime.build_tasks.is_empty());
    }

    #[test]
    fn building_placeholder_cannot_be_removed_as_a_regular_element() {
        let mut runtime = Runtime::demo();
        runtime
            .submit_build("surface-desktop", "创建天气应用")
            .unwrap();

        assert!(
            runtime
                .remove_element("surface-desktop", "build-placeholder-1")
                .is_err()
        );
        assert_eq!(runtime.snapshot().revision, 2);
        assert_eq!(runtime.build_tasks.len(), 1);
    }
}
