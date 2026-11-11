// Runs at the head of `Game::native_level_tick_pre` (API 4): every due label in
// this level acts, as upstream's `LevelTickEvent.Pre` handler does.
fn transfer_labels_level_tick_pre(game: &mut crate::game::Game<'_>, dimension: &str) {
    crate::game::transfer_labels_state::on_level_tick_pre(game, dimension);
}
