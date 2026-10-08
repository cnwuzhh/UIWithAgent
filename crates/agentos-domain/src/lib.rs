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
    pub columns: u8,
    pub rows: u8,
    pub elements: Vec<Element>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GridRect {
    pub x: u8,
    pub y: u8,
    pub width: u8,
    pub height: u8,
}

impl GridRect {
    pub fn new(x: u8, y: u8, width: u8, height: u8) -> Result<Self, DomainError> {
        if width == 0 || height == 0 {
            return Err(DomainError::InvalidRect);
        }
        Ok(Self {
            x,
            y,
            width,
            height,
        })
    }

    fn fits(self, columns: u8, rows: u8) -> bool {
        u16::from(self.x) + u16::from(self.width) <= u16::from(columns)
            && u16::from(self.y) + u16::from(self.height) <= u16::from(rows)
    }

    fn intersects(self, other: Self) -> bool {
        self.x < other.x.saturating_add(other.width)
            && other.x < self.x.saturating_add(self.width)
            && self.y < other.y.saturating_add(other.height)
            && other.y < self.y.saturating_add(self.height)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Element {
    TimePanel(TimePanel),
    AppIcon(AppIcon),
    TextPanel(TextPanel),
}

impl Element {
    pub fn id(&self) -> &str {
        match self {
            Self::TimePanel(element) => &element.id,
            Self::AppIcon(element) => &element.id,
            Self::TextPanel(element) => &element.id,
        }
    }

    pub fn rect(&self) -> GridRect {
        match self {
            Self::TimePanel(element) => element.rect,
            Self::AppIcon(element) => element.rect,
            Self::TextPanel(element) => element.rect,
        }
    }

    fn set_rect(&mut self, rect: GridRect) {
        match self {
            Self::TimePanel(element) => element.rect = rect,
            Self::AppIcon(element) => element.rect = rect,
            Self::TextPanel(element) => element.rect = rect,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TimePanel {
    pub id: String,
    pub title: String,
    pub timezone: String,
    pub rect: GridRect,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AppIcon {
    pub id: String,
    pub title: String,
    pub target_surface_id: SurfaceId,
    pub rect: GridRect,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TextPanel {
    pub id: String,
    pub title: String,
    pub runs: Vec<TextRun>,
    pub rect: GridRect,
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

    pub fn reposition_element(
        &self,
        surface_id: &SurfaceId,
        element_id: &str,
        x: u8,
        y: u8,
    ) -> Result<Self, DomainError> {
        let surface = self.surface(surface_id)?;
        let element = surface
            .elements
            .iter()
            .find(|element| element.id() == element_id)
            .ok_or_else(|| DomainError::ElementNotFound(element_id.into()))?;
        let current = element.rect();
        let candidate = GridRect::new(x, y, current.width, current.height)?;
        Self::validate_element_rect(surface, element_id, candidate)?;

        let mut document = self.clone();
        let element = document
            .surfaces
            .get_mut(surface_id)
            .and_then(|surface| {
                surface
                    .elements
                    .iter_mut()
                    .find(|element| element.id() == element_id)
            })
            .ok_or_else(|| DomainError::ElementNotFound(element_id.into()))?;
        element.set_rect(candidate);
        document.revision += 1;
        Ok(document)
    }

    pub fn resize_element(
        &self,
        surface_id: &SurfaceId,
        element_id: &str,
        width: u8,
        height: u8,
    ) -> Result<Self, DomainError> {
        let surface = self.surface(surface_id)?;
        let element = surface
            .elements
            .iter()
            .find(|element| element.id() == element_id)
            .ok_or_else(|| DomainError::ElementNotFound(element_id.into()))?;
        let current = element.rect();
        let candidate = GridRect::new(current.x, current.y, width, height)?;
        Self::validate_element_rect(surface, element_id, candidate)?;

        let mut document = self.clone();
        let element = document
            .surfaces
            .get_mut(surface_id)
            .and_then(|surface| {
                surface
                    .elements
                    .iter_mut()
                    .find(|element| element.id() == element_id)
            })
            .ok_or_else(|| DomainError::ElementNotFound(element_id.into()))?;
        element.set_rect(candidate);
        document.revision += 1;
        Ok(document)
    }

    fn validate(&self) -> Result<(), DomainError> {
        let root = self.surface(&self.root_surface_id)?;
        if root.parent_id.is_some() {
            return Err(DomainError::RootHasParent);
        }

        let mut element_ids = HashSet::new();
        for surface in self.surfaces.values() {
            if surface.columns == 0 || surface.rows == 0 {
                return Err(DomainError::InvalidSurfaceBounds(surface.id.clone()));
            }
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

            for (index, element) in surface.elements.iter().enumerate() {
                if !element_ids.insert(element.id()) {
                    return Err(DomainError::DuplicateElement(element.id().into()));
                }
                Self::validate_element_rect(surface, element.id(), element.rect())?;
                for other in surface.elements.iter().skip(index + 1) {
                    if element.rect().intersects(other.rect()) {
                        return Err(DomainError::ElementCollision {
                            first: element.id().into(),
                            second: other.id().into(),
                        });
                    }
                }

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

    fn validate_element_rect(
        surface: &Surface,
        element_id: &str,
        candidate: GridRect,
    ) -> Result<(), DomainError> {
        if !candidate.fits(surface.columns, surface.rows) {
            return Err(DomainError::ElementOutOfBounds(element_id.into()));
        }
        if let Some(other) = surface
            .elements
            .iter()
            .find(|other| other.id() != element_id && candidate.intersects(other.rect()))
        {
            return Err(DomainError::ElementCollision {
                first: element_id.into(),
                second: other.id().into(),
            });
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
    DuplicateElement(String),
    DuplicateSurface(SurfaceId),
    ElementCollision { first: String, second: String },
    ElementNotFound(String),
    ElementOutOfBounds(String),
    EmptySurfaceId,
    InvalidRect,
    InvalidSurfaceBounds(SurfaceId),
    MissingParent(SurfaceId),
    MissingTarget(SurfaceId),
    RootHasParent,
    SurfaceCycle(SurfaceId),
    SurfaceNotFound(SurfaceId),
}

impl fmt::Display for DomainError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateElement(id) => write!(formatter, "duplicate element: {id}"),
            Self::DuplicateSurface(id) => write!(formatter, "duplicate surface: {}", id.as_str()),
            Self::ElementCollision { first, second } => {
                write!(formatter, "element collision: {first} with {second}")
            }
            Self::ElementNotFound(id) => write!(formatter, "element not found: {id}"),
            Self::ElementOutOfBounds(id) => write!(formatter, "element out of bounds: {id}"),
            Self::EmptySurfaceId => formatter.write_str("surface id cannot be empty"),
            Self::InvalidRect => formatter.write_str("rect width and height must be positive"),
            Self::InvalidSurfaceBounds(id) => {
                write!(
                    formatter,
                    "surface bounds must be positive: {}",
                    id.as_str()
                )
            }
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
    use super::{
        Element, GridRect, GuiDocument, Surface, SurfaceId, TextPanel, TextRun, TimePanel,
    };

    fn surface(id: &str, parent_id: Option<&str>) -> Surface {
        Surface {
            id: SurfaceId::new(id).unwrap(),
            parent_id: parent_id.map(|value| SurfaceId::new(value).unwrap()),
            title: id.into(),
            icon: "layout".into(),
            columns: 12,
            rows: 8,
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
            rect: GridRect::new(0, 0, 6, 1).unwrap(),
        }));

        let result = GuiDocument::new(1, SurfaceId::new("desktop").unwrap(), vec![desktop]);

        assert!(result.is_err());
    }

    #[test]
    fn reposition_is_immutable_and_increments_revision() {
        let mut desktop = surface("desktop", None);
        desktop.elements.push(Element::TimePanel(TimePanel {
            id: "clock".into(),
            title: "Clock".into(),
            timezone: "local".into(),
            rect: GridRect::new(0, 0, 4, 1).unwrap(),
        }));
        let document =
            GuiDocument::new(1, SurfaceId::new("desktop").unwrap(), vec![desktop]).unwrap();

        let moved = document
            .reposition_element(&SurfaceId::new("desktop").unwrap(), "clock", 4, 2)
            .unwrap();

        assert_eq!(document.revision(), 1);
        assert_eq!(
            document
                .surface(&SurfaceId::new("desktop").unwrap())
                .unwrap()
                .elements[0]
                .rect()
                .x,
            0
        );
        assert_eq!(moved.revision(), 2);
        assert_eq!(
            moved
                .surface(&SurfaceId::new("desktop").unwrap())
                .unwrap()
                .elements[0]
                .rect()
                .x,
            4
        );
    }

    #[test]
    fn reposition_rejects_collision_and_out_of_bounds() {
        let mut desktop = surface("desktop", None);
        for (id, x) in [("left", 0), ("right", 4)] {
            desktop.elements.push(Element::TimePanel(TimePanel {
                id: id.into(),
                title: id.into(),
                timezone: "local".into(),
                rect: GridRect::new(x, 0, 4, 1).unwrap(),
            }));
        }
        let document =
            GuiDocument::new(1, SurfaceId::new("desktop").unwrap(), vec![desktop]).unwrap();
        let id = SurfaceId::new("desktop").unwrap();

        assert!(document.reposition_element(&id, "left", 4, 0).is_err());
        assert!(document.reposition_element(&id, "left", 10, 0).is_err());
    }

    #[test]
    fn resize_is_immutable_and_increments_revision() {
        let mut desktop = surface("desktop", None);
        desktop.elements.push(Element::TimePanel(TimePanel {
            id: "clock".into(),
            title: "Clock".into(),
            timezone: "local".into(),
            rect: GridRect::new(0, 0, 2, 1).unwrap(),
        }));
        let document =
            GuiDocument::new(1, SurfaceId::new("desktop").unwrap(), vec![desktop]).unwrap();
        let id = SurfaceId::new("desktop").unwrap();

        let resized = document.resize_element(&id, "clock", 4, 2).unwrap();

        assert_eq!(document.surface(&id).unwrap().elements[0].rect().width, 2);
        assert_eq!(resized.revision(), 2);
        assert_eq!(resized.surface(&id).unwrap().elements[0].rect().width, 4);
        assert_eq!(resized.surface(&id).unwrap().elements[0].rect().height, 2);
    }

    #[test]
    fn resize_rejects_zero_collision_and_out_of_bounds() {
        let mut desktop = surface("desktop", None);
        for (id, x) in [("left", 0), ("right", 4)] {
            desktop.elements.push(Element::TimePanel(TimePanel {
                id: id.into(),
                title: id.into(),
                timezone: "local".into(),
                rect: GridRect::new(x, 0, 2, 1).unwrap(),
            }));
        }
        let document =
            GuiDocument::new(1, SurfaceId::new("desktop").unwrap(), vec![desktop]).unwrap();
        let id = SurfaceId::new("desktop").unwrap();

        assert!(document.resize_element(&id, "left", 0, 1).is_err());
        assert!(document.resize_element(&id, "left", 5, 1).is_err());
        assert!(document.resize_element(&id, "right", 9, 1).is_err());
        assert_eq!(document.revision(), 1);
    }

    #[test]
    fn rejects_empty_surface_bounds() {
        let mut desktop = surface("desktop", None);
        desktop.rows = 0;

        let result = GuiDocument::new(1, SurfaceId::new("desktop").unwrap(), vec![desktop]);

        assert!(result.is_err());
    }
}
