// Runs at the head of `Game::native_left_click_block` (API 30): a label hit with
// the label item in hand comes off and is given back.
fn transfer_labels_left_click_block(
    game: &mut crate::game::Game<'_>,
    pos: [i32; 3],
    face: [i32; 3],
    canceled: &mut bool,
) {
    crate::game::transfer_labels_state::on_left_click_block(game, pos, face, canceled);
}
