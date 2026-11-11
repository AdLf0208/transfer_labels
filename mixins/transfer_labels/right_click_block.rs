// Runs at the head of `Game::native_right_click_block` (API 21): a label item
// in hand sticks a label to the clicked face and is used up.
fn transfer_labels_right_click_block(
    game: &mut crate::game::Game<'_>,
    pos: [i32; 3],
    face: [i32; 3],
    canceled: &mut bool,
) {
    crate::game::transfer_labels_state::on_right_click_block(game, pos, face, canceled);
}
