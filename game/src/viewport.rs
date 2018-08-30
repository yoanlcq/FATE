use std::collections::HashMap;

use fate::math::{Rect, Rgba};

use rand::random;

use mouse_cursor::{MouseCursor, SystemCursor};
use system::*;

#[derive(Debug)]
pub struct ViewportDB {
    border_color: Rgba<f32>,
    border_px: u32,
    highest_id: ViewportNodeID, // Do not keep; Replace by SlotMap!
    root: ViewportNodeID,
    focused: ViewportNodeID,
    hovered: Option<ViewportNodeID>,
    nodes: HashMap<ViewportNodeID, ViewportNode>,
}

#[derive(Debug, Default, Copy, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct ViewportNodeID(u32);

#[derive(Debug, Clone, PartialEq)]
pub enum ViewportNode {
    Whole {
        parent: Option<ViewportNodeID>,
        info: ViewportInfo
    },
    Split {
        parent: Option<ViewportNodeID>,
        split: Split,
        children: (ViewportNodeID, ViewportNodeID),
    }
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct ViewportInfo {
    // TODO: Describes what a viewport displays    
    pub clear_color: Rgba<f32>,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Split {
    pub origin: SplitOrigin,
    pub unit: SplitUnit,
    pub value: f32,
    pub direction: SplitDirection,
}

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum SplitOrigin {
    LeftOrBottom, Middle, RightOrTop,    
}

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum SplitUnit {
    Ratio, Px,
}

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum SplitDirection {
    Horizontal, Vertical,
}

pub trait ViewportVisitor {
    fn accept_leaf_viewport(&mut self, AcceptLeafViewport);
    fn accept_split_viewport(&mut self, AcceptSplitViewport);
}

#[derive(Debug)]
pub struct AcceptLeafViewport<'a> {
    pub id: ViewportNodeID,
    pub rect: Rect<u32, u32>, 
    pub info: &'a mut ViewportInfo,
    pub parent: Option<ViewportNodeID>,
    pub border_px: u32,
}
#[derive(Debug)]
pub struct AcceptSplitViewport<'a> {
    pub id: ViewportNodeID,
    pub rect: Rect<u32, u32>, 
    pub split_direction: SplitDirection,
    pub distance_from_left_or_bottom_px: &'a mut u32,
    pub parent: Option<ViewportNodeID>,
    pub border_px: u32,
}

#[derive(Debug)]
pub struct ViewportInputHandler;

#[derive(Debug)]
struct ViewportPicker {
    pos: Vec2<u32>,
    found: Option<ViewportNodeID>,
    on_border: Option<ViewportNodeID>,
}



impl ViewportInputHandler {
    pub fn new() -> Self {
        ViewportInputHandler 
    }
}

impl Default for ViewportNode {
    fn default() -> Self {
        ViewportNode::Whole {
            parent: None,
            info: Default::default(),
        }
    }
}

impl ViewportDB {
    pub fn new() -> Self {
        let mut nodes = HashMap::new();
        let root = ViewportNodeID(0);
        nodes.insert(root, ViewportNode::default());
        let highest_id = root;
 
        Self {
            nodes,
            highest_id,
            root,
            focused: root,
            hovered: None,
            border_px: 1,
            border_color: Rgba::grey(0.96),
        }
    }
}


impl System for ViewportInputHandler {
    fn on_mouse_motion(&mut self, g: &mut G, pos: Vec2<f64>) {
        // TODO: Update g.hovered_viewport_node and g.focused_viewport_node.
        g.mouse_cursor = MouseCursor::System(SystemCursor::Hand);

        let mut pos = pos.map(|x| x.round() as u32);
        pos.y = g.input.canvas_size().h.saturating_sub(pos.y);
        let mut visitor = ViewportPicker { pos, found: None, on_border: None, };
        g.visit_viewports(&mut visitor);
        g.viewport_db_mut().hover(visitor.found);
    }
    fn on_mouse_leave(&mut self, g: &mut G) {
        g.viewport_db_mut().hover(None);
    }
    fn on_mouse_button(&mut self, g: &mut G, btn: MouseButton, state: ButtonState) {
        match btn {
            MouseButton::Left if state.is_down() => {
                if let Some(hovered) = g.viewport_db().hovered() {
                    g.viewport_db_mut().focus(hovered);
                }
            },
            _ => {},
        }
    }
    fn on_key(&mut self, g: &mut G, key: Key, state: KeyState) {
        match key.sym {
            Some(Keysym::V) if state.is_down() => g.viewport_db_mut().split_v(),
            Some(Keysym::H) if state.is_down() => g.viewport_db_mut().split_h(),
            Some(Keysym::M) if state.is_down() => g.viewport_db_mut().merge(),
            _ => {},
        }
    }
}

impl ViewportVisitor for ViewportPicker {
    fn accept_leaf_viewport(&mut self, args: AcceptLeafViewport) {
        if args.rect.contains_point(self.pos) {
            self.found = Some(args.id);
        }
    }
    fn accept_split_viewport(&mut self, args: AcceptSplitViewport) {
        unimplemented!()
    }
}


impl ViewportDB {
    pub fn border_color(&self) -> Rgba<f32> {
        self.border_color
    }
    pub fn border_px(&self) -> u32 {
        self.border_px
    }
    pub fn root(&self) -> ViewportNodeID {
        self.root
    }
    pub fn hovered(&self) -> Option<ViewportNodeID> {
        self.hovered
    }
    pub fn hover(&mut self, id: Option<ViewportNodeID>) {
        debug!("Now hovering {:?}", id);
        self.hovered = id;
    }
    pub fn focused(&self) -> ViewportNodeID {
        self.focused
    }
    pub fn focus(&mut self, id: ViewportNodeID) {
        debug!("Now focusing {:?}", id);
        self.focused = id;
    }
    pub fn node(&self, id: ViewportNodeID) -> Option<&ViewportNode> {
        self.nodes.get(&id)
    }
    pub fn node_mut(&mut self, id: ViewportNodeID) -> Option<&mut ViewportNode> {
        self.nodes.get_mut(&id)
    }
    pub fn split_h(&mut self) {
        self.split(SplitDirection::Horizontal)
    }
    pub fn split_v(&mut self) {
        self.split(SplitDirection::Vertical)
    }
    pub fn split(&mut self, direction: SplitDirection) {
        let id = self.focused();

        let c0_id = ViewportNodeID(self.highest_id.0 + 1);
        let c1_id = ViewportNodeID(self.highest_id.0 + 2);

        let info = {
            let node = self.node_mut(id).unwrap();
            let (parent, info) = match *node {
                ViewportNode::Split { .. } => panic!("A non-leaf viewport node cannot be focused"),
                ViewportNode::Whole { ref info, parent, .. } => (parent, info.clone()),
            };
            *node = ViewportNode::Split {
                parent,
                children: (c0_id, c1_id),
                split: Split {
                    direction,
                    origin: SplitOrigin::Middle,
                    unit: SplitUnit::Ratio,
                    value: 0.,
                }
            };
            info
        };

        self.highest_id.0 += 2;
        let c0_info = info.clone();
        let mut c1_info = info;

        self.focus(c0_id);
        c1_info.clear_color = Rgba::<u8>::new_opaque(random(), random(), random()).map(|x| x as f32 / 255.);

        let c0_node = ViewportNode::Whole { info: c0_info, parent: Some(id) };
        let c1_node = ViewportNode::Whole { info: c1_info, parent: Some(id) };
        self.nodes.insert(c0_id, c0_node);
        self.nodes.insert(c1_id, c1_node);
    }
    /// Merges the focused viewport node into its neighbour.
    pub fn merge(&mut self) {
        let focus_id = self.focused();

        let (merge_id, info) = {
            let focus = self.node_mut(focus_id).unwrap();
            let (parent, info) = match *focus {
                ViewportNode::Split { .. } => panic!("A non-leaf viewport node cannot be focused"),
                ViewportNode::Whole { parent, ref info } => (parent, info.clone()),
            };
            (parent, info)
        };

        let merge_id = match merge_id {
            None => return,
            Some(x) => x,
        };

        let (c0_id, c1_id) = {
            let merge = self.node_mut(merge_id).unwrap();
            let (parent, c0_id, c1_id) = match *merge {
                ViewportNode::Whole { .. } => panic!("A parent node can't be whole"),
                ViewportNode::Split { parent, children, .. } => (parent, children.0, children.1),
            };
            *merge = ViewportNode::Whole { info, parent };
            (c0_id, c1_id)
        };

        self.nodes.remove(&c0_id).unwrap();
        self.nodes.remove(&c1_id).unwrap();
        self.focus(merge_id);
    }
    pub fn visit(&mut self, rect: Rect<u32, u32>, f: &mut ViewportVisitor) {
        let root_id = self.root();
        let border_px = self.border_px();
        self.visit_viewport(root_id, rect, f, border_px)
    }
    fn visit_viewport(&mut self, id: ViewportNodeID, rect: Rect<u32, u32>, f: &mut ViewportVisitor, border_px: u32) {
        let (c0, c1, r0, r1) = {
            let node = self.node_mut(id).unwrap();
            match *node {
                ViewportNode::Split { children: (c0, c1), split: Split { origin, unit, ref mut value, direction }, parent } => {
                    // FIXME: assuming value is relative to middle
                    let mut r0 = rect;
                    let mut r1 = rect;
                    let mut distance_from_left_or_bottom_px = match direction {
                        SplitDirection::Horizontal => {
                            r0.h /= 2;
                            r1.h = rect.h - r0.h;
                            r1.y = rect.y + r0.h;
                            r1.y
                        },
                        SplitDirection::Vertical => {
                            r0.w /= 2;
                            r1.w = rect.w - r0.w;
                            r1.x = rect.x + r0.w;
                            r1.x
                        },
                    };
                    f.accept_split_viewport(AcceptSplitViewport{ id, rect, split_direction: direction, distance_from_left_or_bottom_px: &mut distance_from_left_or_bottom_px, parent, border_px });
                    // FIXME: Take mutations of distance_... into account
                    (c0, c1, r0, r1)
                },
                ViewportNode::Whole { ref mut info, parent } => {
                    let border_px = if parent.is_some() {
                        border_px
                    } else {
                        0
                    };
                    return f.accept_leaf_viewport(AcceptLeafViewport{ id, rect, info, parent, border_px });
                },
            }
        };
        self.visit_viewport(c0, r0, f, border_px);
        self.visit_viewport(c1, r1, f, border_px);
    }
}