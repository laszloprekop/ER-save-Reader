//! Column definition for unified tables.

use crate::ui::tokens::dimensions;

/// Text alignment for column content
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Alignment {
    #[default]
    Left,
    Center,
    Right,
}

/// Sort direction for sortable columns
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum SortDirection {
    #[default]
    Ascending,
    Descending,
}

impl SortDirection {
    pub fn toggle(&self) -> Self {
        match self {
            SortDirection::Ascending => SortDirection::Descending,
            SortDirection::Descending => SortDirection::Ascending,
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            SortDirection::Ascending => egui_phosphor::regular::CARET_UP,
            SortDirection::Descending => egui_phosphor::regular::CARET_DOWN,
        }
    }
}

/// Column width specification
#[derive(Clone, Copy, Debug)]
pub enum ColumnWidth {
    /// Fixed width in pixels
    Fixed(f32),
    /// Fraction of remaining space (0.0 - 1.0)
    Fraction(f32),
    /// Auto-size based on content
    Auto,
}

impl Default for ColumnWidth {
    fn default() -> Self {
        ColumnWidth::Auto
    }
}

/// Definition of a table column
#[derive(Clone)]
pub struct Column {
    /// Internal identifier for the column
    pub id: String,
    /// Display header text
    pub header: String,
    /// Width specification
    pub width: ColumnWidth,
    /// Minimum width (for resizable columns)
    pub min_width: f32,
    /// Whether the column is sortable
    pub sortable: bool,
    /// Whether to use monospace font
    pub monospace: bool,
    /// Text alignment
    pub alignment: Alignment,
    /// Whether the column is initially visible
    pub visible: bool,
    /// Whether the column contains icons (renders at 150% size)
    pub icon: bool,
}

impl Column {
    /// Create a new column with required parameters
    pub fn new(id: impl Into<String>, header: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            header: header.into(),
            width: ColumnWidth::Auto,
            min_width: dimensions::TABLE_COLUMN_MIN_WIDTH,
            sortable: false,
            monospace: false,
            alignment: Alignment::Left,
            visible: true,
            icon: false,
        }
    }

    /// Set fixed width in pixels
    pub fn width(mut self, width: f32) -> Self {
        self.width = ColumnWidth::Fixed(width);
        self
    }

    /// Set width as fraction of remaining space
    pub fn width_fraction(mut self, fraction: f32) -> Self {
        self.width = ColumnWidth::Fraction(fraction.clamp(0.0, 1.0));
        self
    }

    /// Set minimum width (for resizable columns)
    pub fn min_width(mut self, min_width: f32) -> Self {
        self.min_width = min_width;
        self
    }

    /// Enable sorting for this column
    pub fn sortable(mut self, sortable: bool) -> Self {
        self.sortable = sortable;
        self
    }

    /// Use monospace font for this column
    pub fn monospace(mut self, monospace: bool) -> Self {
        self.monospace = monospace;
        self
    }

    /// Set text alignment
    pub fn align(mut self, alignment: Alignment) -> Self {
        self.alignment = alignment;
        self
    }

    /// Center-align text
    pub fn center(self) -> Self {
        self.align(Alignment::Center)
    }

    /// Right-align text
    pub fn right(self) -> Self {
        self.align(Alignment::Right)
    }

    /// Set initial visibility
    pub fn visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }

    /// Mark column as containing icons (renders at 150% size)
    pub fn icon(mut self) -> Self {
        self.icon = true;
        self
    }
}
