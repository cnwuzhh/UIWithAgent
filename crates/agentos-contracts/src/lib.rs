use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

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
pub struct BuildTaskDto {
    pub id: String,
    pub surface_id: String,
    pub request: String,
    pub status: String,
    pub agent_message: String,
    pub placeholder_element_id: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentPlanDto {
    pub mode: String,
    pub request: String,
    pub goal: String,
    pub approach: String,
    pub architecture: Vec<String>,
    pub data_contract: String,
    pub build_commands: Vec<String>,
    pub run_command: String,
    pub steps: Vec<AgentPlanStepDto>,
    pub ui_binding: AgentPlanUiBindingDto,
    pub impact: String,
    pub assumptions: Vec<String>,
    pub capabilities: Vec<String>,
    pub based_on_revision: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentPlanUiBindingDto {
    pub type_id: String,
    pub config: BTreeMap<String, AgentPlanConfigValueDto>,
    pub adapter_id: Option<String>,
    pub status: String,
    pub notes: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum AgentPlanConfigValueDto {
    String(String),
    StringList(Vec<String>),
    Boolean(bool),
    Integer(i64),
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentPlanStepDto {
    pub title: String,
    pub action: String,
    pub deliverables: Vec<String>,
    pub verification: String,
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
    use super::{
        AgentPlanConfigValueDto, AgentPlanDto, AgentPlanStepDto, AgentPlanUiBindingDto, ElementDto,
        GridRectDto, TextRunDto,
    };
    use serde_json::json;
    use std::collections::BTreeMap;

    #[test]
    fn agent_plan_is_explicitly_read_only() {
        let plan = AgentPlanDto {
            mode: "planOnly".into(),
            request: "增加当地天气".into(),
            goal: "显示当地天气".into(),
            approach: "生成独立天气数据程序，并将输出绑定到已注册元素。".into(),
            architecture: vec!["Fetcher：获取并规范化天气数据。".into()],
            data_contract: "stdout 输出 recordBatch JSONL，output=weather。".into(),
            build_commands: vec!["python -m pip install -r requirements.txt".into()],
            run_command: "python -m weather_app".into(),
            steps: vec![AgentPlanStepDto {
                title: "定义数据获取".into(),
                action: "创建天气 API client 并规范化响应。".into(),
                deliverables: vec!["src/weather.ts".into(), "天气输出 JSON Schema".into()],
                verification: "运行 fixture 测试并验证输出符合 Schema。".into(),
            }],
            ui_binding: AgentPlanUiBindingDto {
                type_id: "text.panel".into(),
                config: BTreeMap::from([
                    (
                        "title".into(),
                        AgentPlanConfigValueDto::String("当地天气".into()),
                    ),
                    (
                        "runs".into(),
                        AgentPlanConfigValueDto::StringList(vec!["天气摘要".into()]),
                    ),
                ]),
                adapter_id: Some("weather-summary-to-text-panel".into()),
                status: "blocked".into(),
                notes: "等待 Host 注册动态数据适配器。".into(),
            },
            impact: "计划新增一个面板，不执行变更。".into(),
            assumptions: vec!["用户允许联网获取天气".into()],
            capabilities: vec!["surface.read".into()],
            based_on_revision: 2,
        };

        assert_eq!(
            serde_json::to_value(plan).unwrap(),
            json!({
                "mode": "planOnly",
                "request": "增加当地天气",
                "goal": "显示当地天气",
                "approach": "生成独立天气数据程序，并将输出绑定到已注册元素。",
                "architecture": ["Fetcher：获取并规范化天气数据。"],
                "dataContract": "stdout 输出 recordBatch JSONL，output=weather。",
                "buildCommands": ["python -m pip install -r requirements.txt"],
                "runCommand": "python -m weather_app",
                "steps": [{
                    "title": "定义数据获取",
                    "action": "创建天气 API client 并规范化响应。",
                    "deliverables": ["src/weather.ts", "天气输出 JSON Schema"],
                    "verification": "运行 fixture 测试并验证输出符合 Schema。"
                }],
                "uiBinding": {
                    "typeId": "text.panel",
                    "config": {"runs": ["天气摘要"], "title": "当地天气"},
                    "adapterId": "weather-summary-to-text-panel",
                    "status": "blocked",
                    "notes": "等待 Host 注册动态数据适配器。"
                },
                "impact": "计划新增一个面板，不执行变更。",
                "assumptions": ["用户允许联网获取天气"],
                "capabilities": ["surface.read"],
                "basedOnRevision": 2
            })
        );
    }

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
