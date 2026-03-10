use std::collections::BTreeMap;

use ftui::core::geometry::Rect;
use ftui::layout::pane::{
    PaneConstraints, PaneId, PaneLayout, PaneLeaf, PaneNodeKind, PaneNodeRecord, PaneSplit,
    PaneSplitRatio, PaneTree, PaneTreeSnapshot, SplitAxis,
};

use super::{HEADER_HEIGHT, STATUS_HEIGHT};

/// Semantic identity for each Grove pane region.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(not(test), allow(dead_code))]
pub(super) enum PaneRole {
    Header,
    Workspace,
    WorkspaceList,
    Preview,
    Status,
}

#[cfg_attr(not(test), allow(dead_code))]
impl PaneRole {
    /// All roles that must be present in a valid Grove pane tree.
    pub(super) const ALL: &[PaneRole] = &[
        PaneRole::Header,
        PaneRole::Workspace,
        PaneRole::WorkspaceList,
        PaneRole::Preview,
        PaneRole::Status,
    ];

    fn surface_key(self) -> &'static str {
        match self {
            PaneRole::Header => "header",
            PaneRole::Workspace => "workspace",
            PaneRole::WorkspaceList => "workspace_list",
            PaneRole::Preview => "preview",
            PaneRole::Status => "status",
        }
    }
}

/// Grove-owned pane adapter wrapping ftui `PaneTree` with semantic role access.
#[cfg_attr(not(test), allow(dead_code))]
pub(super) struct GrovePaneModel {
    tree: PaneTree,
    role_ids: BTreeMap<PaneRole, PaneId>,
}

#[cfg_attr(not(test), allow(dead_code))]
impl GrovePaneModel {
    /// Build the canonical full-screen pane tree.
    ///
    /// Structure:
    /// ```text
    /// root (vertical split)
    /// ├─ header_status_split (vertical split)
    /// │  ├─ header (leaf, fixed 1 row)
    /// │  └─ workspace (horizontal split)
    /// │     ├─ workspace_list (leaf)
    /// │     └─ preview (leaf)
    /// └─ status (leaf, fixed 1 row)
    /// ```
    pub(super) fn canonical(sidebar_width_pct: u16) -> Self {
        // IDs: 1=root, 2=header_workspace_split, 3=status,
        //      4=header, 5=workspace, 6=workspace_list, 7=preview
        let root_id = PaneId::new(1).expect("valid id");
        let header_workspace_id = PaneId::new(2).expect("valid id");
        let status_id = PaneId::new(3).expect("valid id");
        let header_id = PaneId::new(4).expect("valid id");
        let workspace_id = PaneId::new(5).expect("valid id");
        let workspace_list_id = PaneId::new(6).expect("valid id");
        let preview_id = PaneId::new(7).expect("valid id");
        let next_id = PaneId::new(8).expect("valid id");

        // Root: vertical split, header+workspace vs status.
        // Ratio heavily favors the first child (header+workspace area).
        // The constraint on status enforces exactly STATUS_HEIGHT rows.
        let root = PaneNodeRecord {
            id: root_id,
            parent: None,
            constraints: no_margin_constraints(),
            kind: PaneNodeKind::Split(PaneSplit {
                axis: SplitAxis::Vertical,
                ratio: PaneSplitRatio::new(99, 1).expect("valid ratio"),
                first: header_workspace_id,
                second: status_id,
            }),
            extensions: BTreeMap::new(),
        };

        let status = PaneNodeRecord {
            id: status_id,
            parent: Some(root_id),
            constraints: fixed_height_constraints(STATUS_HEIGHT),
            kind: PaneNodeKind::Leaf(PaneLeaf::new(PaneRole::Status.surface_key())),
            extensions: BTreeMap::new(),
        };

        // Header+workspace: vertical split, header vs workspace.
        let header_workspace = PaneNodeRecord {
            id: header_workspace_id,
            parent: Some(root_id),
            constraints: no_margin_constraints(),
            kind: PaneNodeKind::Split(PaneSplit {
                axis: SplitAxis::Vertical,
                ratio: PaneSplitRatio::new(1, 99).expect("valid ratio"),
                first: header_id,
                second: workspace_id,
            }),
            extensions: BTreeMap::new(),
        };

        let header = PaneNodeRecord {
            id: header_id,
            parent: Some(header_workspace_id),
            constraints: fixed_height_constraints(HEADER_HEIGHT),
            kind: PaneNodeKind::Leaf(PaneLeaf::new(PaneRole::Header.surface_key())),
            extensions: BTreeMap::new(),
        };

        // Workspace: horizontal split, list vs preview.
        let sidebar_ratio = sidebar_width_pct.max(1) as u32;
        let preview_ratio = (100u32.saturating_sub(sidebar_ratio)).max(1);
        let workspace = PaneNodeRecord {
            id: workspace_id,
            parent: Some(header_workspace_id),
            constraints: no_margin_constraints(),
            kind: PaneNodeKind::Split(PaneSplit {
                axis: SplitAxis::Horizontal,
                ratio: PaneSplitRatio::new(sidebar_ratio, preview_ratio).expect("valid ratio"),
                first: workspace_list_id,
                second: preview_id,
            }),
            extensions: BTreeMap::new(),
        };

        let workspace_list = PaneNodeRecord {
            id: workspace_list_id,
            parent: Some(workspace_id),
            constraints: no_margin_constraints(),
            kind: PaneNodeKind::Leaf(PaneLeaf::new(PaneRole::WorkspaceList.surface_key())),
            extensions: BTreeMap::new(),
        };

        let preview = PaneNodeRecord {
            id: preview_id,
            parent: Some(workspace_id),
            constraints: no_margin_constraints(),
            kind: PaneNodeKind::Leaf(PaneLeaf::new(PaneRole::Preview.surface_key())),
            extensions: BTreeMap::new(),
        };

        let snapshot = PaneTreeSnapshot {
            schema_version: 1,
            root: root_id,
            next_id,
            nodes: vec![
                root,
                header_workspace,
                status,
                header,
                workspace,
                workspace_list,
                preview,
            ],
            extensions: BTreeMap::new(),
        };

        let tree = PaneTree::from_snapshot(snapshot).expect("canonical tree should be valid");

        let mut role_ids = BTreeMap::new();
        role_ids.insert(PaneRole::Header, header_id);
        role_ids.insert(PaneRole::Workspace, workspace_id);
        role_ids.insert(PaneRole::WorkspaceList, workspace_list_id);
        role_ids.insert(PaneRole::Preview, preview_id);
        role_ids.insert(PaneRole::Status, status_id);

        Self { tree, role_ids }
    }

    /// Solve the pane tree layout for a given viewport.
    pub(super) fn solve(&self, viewport: Rect) -> PaneLayout {
        self.tree
            .solve_layout(viewport)
            .expect("canonical tree solve should always succeed")
    }

    /// Look up the solved rect for a semantic pane role.
    pub(super) fn rect_for_role(&self, layout: &PaneLayout, role: PaneRole) -> Option<Rect> {
        let id = self.role_ids.get(&role)?;
        layout.rect(*id)
    }

    /// Get the PaneId for a semantic role.
    pub(super) fn id_for_role(&self, role: PaneRole) -> Option<PaneId> {
        self.role_ids.get(&role).copied()
    }
}

fn no_margin_constraints() -> PaneConstraints {
    PaneConstraints {
        min_width: 1,
        min_height: 1,
        max_width: None,
        max_height: None,
        collapsible: false,
    }
}

fn fixed_height_constraints(height: u16) -> PaneConstraints {
    PaneConstraints {
        min_height: height,
        max_height: Some(height),
        min_width: 1,
        max_width: None,
        collapsible: false,
    }
}
