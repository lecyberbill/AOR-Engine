// [WFGY] Zone: SAFE | λ: 0.2 | Fallbacks: 0/None | Action: Shared layout style helpers for the studio UI
#![allow(dead_code)]
use ui_layout::{auto, length, AlignItems, Display, FlexDirection, Position, Rect, Size, Style};

pub fn leaf(width: f32, height: f32) -> Style {
    Style {
        size: Size {
            width: length(width),
            height: length(height),
        },
        ..Default::default()
    }
}

pub fn row(gap: f32) -> Style {
    Style {
        display: Display::Flex,
        flex_direction: FlexDirection::Row,
        gap: Size {
            width: length(gap),
            height: length(0.0),
        },
        align_items: Some(AlignItems::Center),
        ..Default::default()
    }
}

pub fn column(gap: f32) -> Style {
    Style {
        display: Display::Flex,
        flex_direction: FlexDirection::Column,
        gap: Size {
            width: length(0.0),
            height: length(gap),
        },
        ..Default::default()
    }
}

pub fn absolute(x: f32, y: f32, w: f32, h: f32) -> Style {
    Style {
        position: Position::Absolute,
        inset: Rect {
            top: length(y),
            left: length(x),
            right: auto(),
            bottom: auto(),
        },
        size: Size {
            width: length(w),
            height: length(h),
        },
        ..Default::default()
    }
}

pub fn popover_style(x: f32, y: f32, w: f32, h: f32) -> Style {
    Style {
        position: Position::Absolute,
        inset: Rect {
            top: length(y),
            left: length(x),
            right: auto(),
            bottom: auto(),
        },
        size: Size {
            width: length(w),
            height: length(h),
        },
        display: Display::Flex,
        flex_direction: FlexDirection::Column,
        gap: Size {
            width: length(0.0),
            height: length(2.0),
        },
        ..Default::default()
    }
}

pub fn fill(width: f32, height: f32) -> Style {
    Style {
        size: Size {
            width: length(width),
            height: length(height),
        },
        ..Default::default()
    }
}
