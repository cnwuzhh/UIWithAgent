use serde::Serialize;

pub const TOOLBOX_VERSION: &str = "2026-10-08.1";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ElementAvailability {
    Available,
    Preview,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ElementCategory {
    Basic,
    Weather,
    Media,
    Calculator,
    Finance,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ConfigValueKind {
    String,
    StringList,
    Boolean,
    Integer,
    Reference,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigFieldDescriptor {
    pub name: &'static str,
    pub value_kind: ConfigValueKind,
    pub required: bool,
    pub allowed_values: &'static [&'static str],
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GridSizeDescriptor {
    pub width: u8,
    pub height: u8,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ElementDescriptor {
    pub type_id: &'static str,
    pub version: u16,
    pub title: &'static str,
    pub description: &'static str,
    pub category: ElementCategory,
    pub availability: ElementAvailability,
    pub default_size: GridSizeDescriptor,
    pub config_fields: &'static [ConfigFieldDescriptor],
    pub required_capabilities: &'static [&'static str],
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecipeDescriptor {
    pub recipe_id: &'static str,
    pub title: &'static str,
    pub required_elements: &'static [&'static str],
    pub optional_elements: &'static [&'static str],
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolboxDescriptor {
    pub version: &'static str,
    pub elements: &'static [ElementDescriptor],
    pub recipes: &'static [RecipeDescriptor],
}

pub trait ToolboxCatalog: Send + Sync {
    fn descriptor(&self) -> ToolboxDescriptor;

    fn describe(&self, type_id: &str) -> Option<&'static ElementDescriptor> {
        self.descriptor()
            .elements
            .iter()
            .find(|element| element.type_id == type_id)
    }

    fn available_element(&self, type_id: &str) -> Option<&'static ElementDescriptor> {
        self.describe(type_id)
            .filter(|element| element.availability == ElementAvailability::Available)
    }

    fn recipe(&self, recipe_id: &str) -> Option<&'static RecipeDescriptor> {
        self.descriptor()
            .recipes
            .iter()
            .find(|recipe| recipe.recipe_id == recipe_id)
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct BuiltInToolboxCatalog;

impl ToolboxCatalog for BuiltInToolboxCatalog {
    fn descriptor(&self) -> ToolboxDescriptor {
        ToolboxDescriptor {
            version: TOOLBOX_VERSION,
            elements: ELEMENTS,
            recipes: RECIPES,
        }
    }
}

const NO_VALUES: &[&str] = &[];

const TIME_FIELDS: &[ConfigFieldDescriptor] = &[
    ConfigFieldDescriptor {
        name: "title",
        value_kind: ConfigValueKind::String,
        required: true,
        allowed_values: NO_VALUES,
    },
    ConfigFieldDescriptor {
        name: "timezone",
        value_kind: ConfigValueKind::String,
        required: true,
        allowed_values: NO_VALUES,
    },
];

const TEXT_FIELDS: &[ConfigFieldDescriptor] = &[
    ConfigFieldDescriptor {
        name: "title",
        value_kind: ConfigValueKind::String,
        required: true,
        allowed_values: NO_VALUES,
    },
    ConfigFieldDescriptor {
        name: "runs",
        value_kind: ConfigValueKind::StringList,
        required: true,
        allowed_values: NO_VALUES,
    },
];

const APP_ICON_FIELDS: &[ConfigFieldDescriptor] = &[
    ConfigFieldDescriptor {
        name: "title",
        value_kind: ConfigValueKind::String,
        required: true,
        allowed_values: NO_VALUES,
    },
    ConfigFieldDescriptor {
        name: "targetSurfaceId",
        value_kind: ConfigValueKind::Reference,
        required: true,
        allowed_values: NO_VALUES,
    },
];

const ELEMENTS: &[ElementDescriptor] = &[
    ElementDescriptor {
        type_id: "time.panel",
        version: 1,
        title: "时间面板",
        description: "显示指定时区的当前时间。",
        category: ElementCategory::Basic,
        availability: ElementAvailability::Available,
        default_size: GridSizeDescriptor {
            width: 4,
            height: 1,
        },
        config_fields: TIME_FIELDS,
        required_capabilities: &[],
    },
    ElementDescriptor {
        type_id: "text.panel",
        version: 1,
        title: "文本面板",
        description: "显示静态文本和受控 Surface 链接。",
        category: ElementCategory::Basic,
        availability: ElementAvailability::Available,
        default_size: GridSizeDescriptor {
            width: 6,
            height: 1,
        },
        config_fields: TEXT_FIELDS,
        required_capabilities: &[],
    },
    ElementDescriptor {
        type_id: "app.icon",
        version: 1,
        title: "应用入口",
        description: "链接到已存在的 Surface。",
        category: ElementCategory::Basic,
        availability: ElementAvailability::Available,
        default_size: GridSizeDescriptor {
            width: 2,
            height: 1,
        },
        config_fields: APP_ICON_FIELDS,
        required_capabilities: &["surface.read"],
    },
    preview(
        "weather.current",
        "当前天气",
        ElementCategory::Weather,
        &["location.read", "weather.read"],
    ),
    preview(
        "weather.forecast",
        "天气预报",
        ElementCategory::Weather,
        &["weather.read"],
    ),
    preview(
        "audio.player",
        "音乐播放器",
        ElementCategory::Media,
        &["media.audio.read", "media.playback"],
    ),
    preview(
        "podcast.player",
        "Podcast 播放器",
        ElementCategory::Media,
        &["podcast.read", "media.playback"],
    ),
    preview(
        "calculator.basic",
        "基础计算器",
        ElementCategory::Calculator,
        &[],
    ),
    preview(
        "video.player",
        "视频播放器",
        ElementCategory::Media,
        &["media.video.read", "media.playback"],
    ),
    preview(
        "stream.player",
        "流媒体播放器",
        ElementCategory::Media,
        &["stream.read", "media.playback"],
    ),
    preview(
        "finance.news",
        "财经新闻",
        ElementCategory::Finance,
        &["finance.news.read"],
    ),
];

const fn preview(
    type_id: &'static str,
    title: &'static str,
    category: ElementCategory,
    required_capabilities: &'static [&'static str],
) -> ElementDescriptor {
    ElementDescriptor {
        type_id,
        version: 1,
        title,
        description: "设计预览；当前不能提交构建。",
        category,
        availability: ElementAvailability::Preview,
        default_size: GridSizeDescriptor {
            width: 4,
            height: 2,
        },
        config_fields: &[],
        required_capabilities,
    }
}

const RECIPES: &[RecipeDescriptor] = &[
    recipe(
        "app.weather.local",
        "当地天气",
        &["weather.current"],
        &["weather.forecast"],
    ),
    recipe("app.music.player", "音乐播放器", &["audio.player"], &[]),
    recipe(
        "app.podcast.player",
        "Podcast 播放器",
        &["podcast.player"],
        &[],
    ),
    recipe("app.calculator", "计算器", &["calculator.basic"], &[]),
    recipe("app.video.player", "视频播放器", &["video.player"], &[]),
    recipe(
        "app.streaming.player",
        "流媒体播放器",
        &["stream.player"],
        &[],
    ),
    recipe("app.finance.news", "财经新闻", &["finance.news"], &[]),
];

const fn recipe(
    recipe_id: &'static str,
    title: &'static str,
    required_elements: &'static [&'static str],
    optional_elements: &'static [&'static str],
) -> RecipeDescriptor {
    RecipeDescriptor {
        recipe_id,
        title,
        required_elements,
        optional_elements,
    }
}

#[cfg(test)]
mod tests {
    use super::{BuiltInToolboxCatalog, ElementAvailability, TOOLBOX_VERSION, ToolboxCatalog};
    use std::collections::HashSet;

    #[test]
    fn catalog_has_stable_unique_ids_and_expected_available_elements() {
        let descriptor = BuiltInToolboxCatalog.descriptor();
        assert_eq!(descriptor.version, TOOLBOX_VERSION);

        let ids = descriptor
            .elements
            .iter()
            .map(|element| element.type_id)
            .collect::<HashSet<_>>();
        assert_eq!(ids.len(), descriptor.elements.len());

        let available = descriptor
            .elements
            .iter()
            .filter(|element| element.availability == ElementAvailability::Available)
            .map(|element| element.type_id)
            .collect::<HashSet<_>>();
        assert_eq!(
            available,
            HashSet::from(["time.panel", "text.panel", "app.icon"])
        );
    }

    #[test]
    fn every_recipe_references_registered_elements() {
        let descriptor = BuiltInToolboxCatalog.descriptor();
        let element_ids = descriptor
            .elements
            .iter()
            .map(|element| element.type_id)
            .collect::<HashSet<_>>();
        let recipe_ids = descriptor
            .recipes
            .iter()
            .map(|recipe| recipe.recipe_id)
            .collect::<HashSet<_>>();

        assert_eq!(recipe_ids.len(), descriptor.recipes.len());
        assert_eq!(descriptor.recipes.len(), 7);
        for recipe in descriptor.recipes {
            for type_id in recipe
                .required_elements
                .iter()
                .chain(recipe.optional_elements.iter())
            {
                assert!(element_ids.contains(type_id), "unknown element {type_id}");
            }
        }
    }

    #[test]
    fn preview_recipe_is_a_design_target_but_not_a_submittable_element() {
        let catalog = BuiltInToolboxCatalog;
        let recipe = catalog.recipe("app.finance.news").unwrap();

        assert_eq!(recipe.required_elements, ["finance.news"]);
        assert_eq!(
            catalog.describe("finance.news").unwrap().availability,
            ElementAvailability::Preview
        );
        assert!(catalog.available_element("finance.news").is_none());
    }

    #[test]
    fn preview_elements_cannot_be_mistaken_for_available_elements() {
        let catalog = BuiltInToolboxCatalog;
        assert_eq!(
            catalog.describe("weather.current").unwrap().availability,
            ElementAvailability::Preview
        );
        assert!(catalog.available_element("weather.current").is_none());
        assert_eq!(
            catalog.available_element("text.panel").unwrap().type_id,
            "text.panel"
        );
        assert!(catalog.describe("weather.unknown").is_none());
        assert!(catalog.available_element("weather.unknown").is_none());
    }
}
