#[expect(
    clippy::struct_field_names,
    reason = "the suffix matches every other *_color name"
)]
pub(crate) struct Theme {
    pub(crate) base_color: u32,
    pub(crate) tile_color: u32,
    pub(crate) selected_color: u32,
    pub(crate) selected_label_text_color: u32,
    pub(crate) cell_marker_border_color: u32,
    pub(crate) cell_marker_fill_color: u32,
    pub(crate) chevron_color: u32,
    pub(crate) selected_chevron_color: u32,
    pub(crate) label_text_color: u32,
}
