use std::collections::{HashMap, HashSet};
use std::fmt;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct SurfaceId(String);

impl SurfaceId {
    pub fn new(value: impl Into<String>) -> Result<Self, DomainError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(DomainError::EmptySurfaceId);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Surface {
    pub id: SurfaceId,
    pub parent_id: Option<SurfaceId>,
    pub title: String,
    pub icon: String,
    pub elements: Vec<Element>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Element {
    TimePanel(TimePanel),
    AppIcon(AppIcon),
    TextPanel(TextPanel),
}

#[derive(Clone, Debug, PartialEq)]
pub struct TimePanel {
    pub id: String,
    pub title: String,
    pub timezone: String,
    pub column: u8,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AppIcon {
    pub id: String,
    pub title: String,
    pub target_surface_id: SurfaceId,
    pub column: u8,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TextPanel {
    pub id: String,
    pub title: String,
    pub runs: Vec<TextRun>,
    pub column: u8,
}

#[derive(Clone, Debug, PartialEq)]
pub enum TextRun {
    Text(String),
    SurfaceLink {
        label: String,
        target_surface_id: SurfaceId,
    },
}

#[derive(Clone, Debug)]
pub struct GuiDocument {
    revision: u64,
    root_surface_id: SurfaceId,
    surfaces: HashMap<SurfaceId, Surface>,
}

impl GuiDocument {
    pub fn new(
        revision: u64,
        root_surface_id: SurfaceId,
        surfaces: Vec<Surface>,
    ) -> Result<Self, DomainError> {
        let mut indexed = HashMap::new();
        for surface in surfaces {
            let id = surface.id.clone();
            if indexed.insert(id.clone(), surface).is_some() {
                return Err(DomainError::DuplicateSurface(id));
            }
        }

        let document = Self {
            revision,
            root_surface_id,
            surfaces: indexed,
        };
        document.validate()?;
        Ok(document)
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn surface(&self, id: &SurfaceId) -> Result<&Surface, DomainError> {
        self.surfaces
            .get(id)
            .ok_or_else(|| DomainError::SurfaceNotFound(id.clone()))
    }

    pub fn path_to(&self, id: &SurfaceId) -> Result<Vec<&Surface>, DomainError> {
        let mut path = Vec::new();
        let mut cursor = self.surface(id)?;
        loop {
            path.push(cursor);
            match &cursor.parent_id {
                Some(parent_id) => cursor = self.surface(parent_id)?,
                None => break,
            }
        }
        path.reverse();
        Ok(path)
    }

    fn validate(&self) -> Result<(), DomainError> {
        let root = self.surface(&self.root_surface_id)?;
        if root.parent_id.is_some() {
            return Err(DomainError::RootHasParent);
        }

        for surface in self.surfaces.values() {
            if surface.id != self.root_surface_id && surface.parent_id.is_none() {
                return Err(DomainError::MissingParent(surface.id.clone()));
            }

            let mut visited = HashSet::new();
            let mut cursor = surface;
            while let Some(parent_id) = &cursor.parent_id {
                if !visited.insert(cursor.id.clone()) {
                    return Err(DomainError::SurfaceCycle(surface.id.clone()));
                }
                cursor = self.surface(parent_id)?;
            }

            for element in &surface.elements {
                match element {
                    Element::AppIcon(icon) => self.validate_target(&icon.target_surface_id)?,
                    Element::TextPanel(panel) => {
                        for run in &panel.runs {
                            if let TextRun::SurfaceLink {
                                target_surface_id, ..
                            } = run
                            {
                                self.validate_target(target_surface_id)?;
                            }
                        }
                    }
                    Element::TimePanel(_) => {}
                }
            }
        }
        Ok(())
    }

    fn validate_target(&self, id: &SurfaceId) -> Result<(), DomainError> {
        self.surface(id)
            .map(|_| ())
            .map_err(|_| DomainError::MissingTarget(id.clone()))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DomainError {
    DuplicateSurface(SurfaceId),
    EmptySurfaceId,
    MissingParent(SurfaceId),
    MissingTarget(SurfaceId),
    RootHasParent,
    SurfaceCycle(SurfaceId),
    SurfaceNotFound(SurfaceId),
}

impl fmt::Display for DomainError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateSurface(id) => write!(formatter, "duplicate surface: {}", id.as_str()),
            Self::EmptySurfaceId => formatter.write_str("surface id cannot be empty"),
            Self::MissingParent(id) => write!(formatter, "surface has no parent: {}", id.as_str()),
            Self::MissingTarget(id) => write!(formatter, "missing surface target: {}", id.as_str()),
            Self::RootHasParent => formatter.write_str("root surface cannot have a parent"),
            Self::SurfaceCycle(id) => write!(formatter, "surface cycle: {}", id.as_str()),
            Self::SurfaceNotFound(id) => write!(formatter, "surface not found: {}", id.as_str()),
        }
    }
}

impl std::error::Error for DomainError {}

#[cfg(test)]
mod tests {
    use super::{Element, GuiDocument, Surface, SurfaceId, TextPanel, TextRun};

    fn surface(id: &str, parent_id: Option<&str>) -> Surface {
        Surface {
            id: SurfaceId::new(id).unwrap(),
            parent_id: parent_id.map(|value| SurfaceId::new(value).unwrap()),
            title: id.into(),
            icon: "layout".into(),
            elements: vec![],
        }
    }

    #[test]
    fn returns_the_unique_root_path() {
        let document = GuiDocument::new(
            1,
            SurfaceId::new("desktop").unwrap(),
            vec![surface("desktop", None), surface("work", Some("desktop"))],
        )
        .unwrap();

        let path = document.path_to(&SurfaceId::new("work").unwrap()).unwrap();
        assert_eq!(
            path.iter().map(|item| item.id.as_str()).collect::<Vec<_>>(),
            ["desktop", "work"]
        );
    }

    #[test]
    fn rejects_cycles() {
        let result = GuiDocument::new(
            1,
            SurfaceId::new("desktop").unwrap(),
            vec![
                surface("desktop", None),
                surface("a", Some("b")),
                surface("b", Some("a")),
            ],
        );

        assert!(result.is_err());
    }

    #[test]
    fn rejects_missing_embedded_text_target() {
        let mut desktop = surface("desktop", None);
        desktop.elements.push(Element::TextPanel(TextPanel {
            id: "welcome".into(),
            title: "Welcome".into(),
            runs: vec![TextRun::SurfaceLink {
                label: "Missing".into(),
                target_surface_id: SurfaceId::new("missing").unwrap(),
            }],
            column: 1,
        }));

        let result = GuiDocument::new(1, SurfaceId::new("desktop").unwrap(), vec![desktop]);

        assert!(result.is_err());
    }
}
