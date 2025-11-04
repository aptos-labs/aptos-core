use crate::file_format_generator::peephole_optimizer::optimizers::{
    TransformedCodeChunk, WindowOptimizer,
};
use move_binary_format::file_format::Bytecode;

pub struct CollapseToDrop;

impl CollapseToDrop {
    const WINDOW_SIZE: usize = 2;
}

impl WindowOptimizer for CollapseToDrop {
    fn optimize_window(&self, window: &[Bytecode]) -> Option<(TransformedCodeChunk, usize)> {
        use Bytecode::*;
        if window.len() < Self::WINDOW_SIZE {
            return None;
        }
        // See module documentation for the reasoning behind these optimizations.
        let optimized = match (&window[0], &window[1]) {
            (MoveLoc(loc), Pop) => TransformedCodeChunk::new(vec![DropLoc(*loc)], vec![0]),
            _ => return None,
        };
        Some((optimized, Self::WINDOW_SIZE))
    }
}

pub struct CollapseToBorrowGetField;

impl CollapseToBorrowGetField {
    const WINDOW_SIZE: usize = 3;
}

impl WindowOptimizer for CollapseToBorrowGetField {
    fn optimize_window(&self, window: &[Bytecode]) -> Option<(TransformedCodeChunk, usize)> {
        use Bytecode::*;
        if window.len() < Self::WINDOW_SIZE {
            return None;
        }
        // See module documentation for the reasoning behind these optimizations.
        let optimized = match (&window[0], &window[1], &window[2]) {
            (
                ImmBorrowLoc(loc_idx),
                ImmBorrowField(field_idx) | MutBorrowField(field_idx),
                ReadRef,
            ) => TransformedCodeChunk::new(
                vec![BorrowGetField(*loc_idx, *field_idx)],
                vec![0],
            ),
            _ => return None,
        };
        Some((optimized, Self::WINDOW_SIZE))
    }
}
