pub(crate) const INTENT_REVIEW_SUMMARY_ROWS: u16 = 2;

const TABLE_HEADER_ROWS: usize = 1;

pub(crate) fn staged_change_index_at_content_row(
    visible_len: usize,
    content_row: usize,
) -> Option<usize> {
    let first_change_row = INTENT_REVIEW_SUMMARY_ROWS as usize + TABLE_HEADER_ROWS;
    let index = content_row.checked_sub(first_change_row)?;
    (index < visible_len).then_some(index)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_row_maps_to_staged_change_index_below_summary_and_header() {
        assert_eq!(staged_change_index_at_content_row(2, 2), None);
        assert_eq!(staged_change_index_at_content_row(2, 3), Some(0));
        assert_eq!(staged_change_index_at_content_row(2, 4), Some(1));
        assert_eq!(staged_change_index_at_content_row(2, 5), None);
    }
}
