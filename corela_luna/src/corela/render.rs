use corela_adahira::Frame;
use corela_ema::entity::{Board, Node};

use super::Corela;
use crate::dto::Theme;

impl Corela {
    pub(super) fn draw(&self, frame: &mut Frame<'_>) {
        let theme = self.theme();
        let board = self.history.board();
        frame.clear(theme.base_color);
        self.draw_status(frame, theme, board);
        let cells = board.cells();
        self.draw_tiles(frame, theme, board, &cells);
        self.draw_nodes(frame, theme, board, &cells);
    }

    fn draw_status(&self, frame: &mut Frame<'_>, theme: &Theme, board: &Board) {
        frame.footer(board.region().name(), theme.label_text_color);
        let mode_label = format!("Mode: {}", self.mode.name());
        frame.above_footer(&mode_label, theme.label_text_color);
        let counter = format!(
            "Action {} / {}",
            self.replay.current_action(),
            self.replay.last_action()
        );
        frame.header(&counter, theme.label_text_color);
    }

    fn draw_tiles(
        &self,
        frame: &mut Frame<'_>,
        theme: &Theme,
        board: &Board,
        cells: &[(i32, i32)],
    ) {
        let selected = board.selected();
        let radius = self.layout.size - self.geometry.hex_tile_inset;
        for cell in cells {
            let (q, r) = *cell;
            let at = self.layout.to_pixel(q, r);
            let active = *cell == selected;
            let color = if active {
                theme.selected_color
            } else {
                theme.tile_color
            };
            frame.hexagon(at, radius, color);
            if board.node_at(*cell).is_none() {
                let text_color = if active {
                    theme.selected_label_text_color
                } else {
                    theme.label_text_color
                };
                frame.label(&board.label(*cell), at, text_color, active);
            }
        }
    }

    fn draw_nodes(
        &self,
        frame: &mut Frame<'_>,
        theme: &Theme,
        board: &Board,
        cells: &[(i32, i32)],
    ) {
        let selected = board.selected();
        for cell in cells {
            let (q, r) = *cell;
            let at = self.layout.to_pixel(q, r);
            match board.node_at(*cell) {
                Some(Node::BpmnStartNote) => {
                    frame.circle(
                        at,
                        self.geometry.cell_marker_outer_radius,
                        theme.cell_marker_border_color,
                    );
                    frame.circle(
                        at,
                        self.geometry.cell_marker_inner_radius,
                        theme.cell_marker_fill_color,
                    );
                }
                Some(Node::Arrow(direction)) => {
                    let angle = f32::from(direction) * std::f32::consts::FRAC_PI_4;
                    let color = if *cell == selected {
                        theme.selected_chevron_color
                    } else {
                        theme.chevron_color
                    };
                    frame.icon(&self.config.chevron_icon, at, angle, color);
                }
                None => {}
            }
        }
    }
}
